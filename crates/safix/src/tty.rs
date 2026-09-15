//! The one terminal a full-screen picker needs, and the one mode it puts it in.
//!
//! This is the only place the command puts a terminal into raw mode.
//! [`prompt`](crate::prompt) remains the only place it toggles echo: a prompt
//! reads one line and wants the line discipline doing its job, where a picker
//! reads single keystrokes and cannot have one.
//!
//! A picker refuses where `prompt` degrades. `prompt` falls back to standard
//! input when `/dev/tty` does not open, because a value read from a pipe is
//! still the value the operator sent; a picker with no terminal has nothing to
//! draw on and no keystrokes to read, so degrading is not a lesser mode but an
//! unusable one, and the refusal is
//! [`Error::PickerNeedsTerminal`](safix_core::Error::PickerNeedsTerminal).

use std::fs::File;
use std::sync::Mutex;

use rustix::termios::{
    InputModes, LocalModes, OptionalActions, SpecialCodeIndex, Termios, tcgetattr, tcsetattr,
};

/// The terminal both this module and [`prompt`](crate::prompt) open.
///
/// One constant and two openers, so the two cannot come to disagree about what
/// they open.
pub(crate) const DEVICE: &str = "/dev/tty";

/// How long the highlight has to rest before the value under it is decrypted.
///
/// Tenths of a second, because that is the unit `VTIME` counts in: the read
/// below returns after this long with nothing rather than blocking, which is
/// what lets one loop serve both keystrokes and a quiet period without a poll.
const READ_DECISECONDS: u8 = 1;

/// The attributes [`Raw`] is holding, for the signal path to put back.
///
/// The guard's [`Drop`] covers return, error and panic. It does not cover
/// [`abort::catch_signals`](crate::abort::catch_signals), which ends the
/// process with `std::process::exit` and runs no destructor, so the saved
/// attributes are reachable from there through [`restore`] as well.
static SAVED: Mutex<Option<Termios>> = Mutex::new(None);

/// The terminal, opened for reading and writing, when there is one.
///
/// Write-opened first and then re-opened read-write, mirroring
/// [`prompt`](crate::prompt)'s own probe, which mirrors the retired shell
/// runtime's `{ : >/dev/tty; } 2>/dev/null`: a controlling terminal that has
/// gone away is a `/dev/tty` that exists as a path and fails to open.
pub(crate) fn probe() -> Option<File> {
    if File::options().write(true).open(DEVICE).is_err() {
        return None;
    }
    File::options().read(true).write(true).open(DEVICE).ok()
}

/// Put back whatever [`Raw`] saved, or do nothing when it saved nothing.
///
/// Called from the signal handler's thread, where no [`Drop`] runs.
pub(crate) fn restore() {
    let Ok(saved) = SAVED.lock() else {
        return;
    };
    let Some(attributes) = saved.as_ref() else {
        return;
    };
    if let Ok(terminal) = File::options().write(true).open(DEVICE) {
        let _ = tcsetattr(&terminal, OptionalActions::Now, attributes);
    }
}

/// Raw mode for as long as this lives.
///
/// [`Silenced`](crate::prompt)'s shape, with one difference: a terminal whose
/// attributes cannot be read is a refusal here where a prompt proceeds. A
/// picker that could not clear `ICANON` would read nothing until the operator
/// pressed return, and a picker that could not clear `ECHO` would draw the
/// query twice.
pub(crate) struct Raw<'a> {
    /// The terminal whose attributes were changed.
    terminal: &'a File,
    /// What they were.
    restore: Termios,
}

impl<'a> Raw<'a> {
    /// Raw mode over this terminal, or nothing when its attributes will not
    /// read.
    pub(crate) fn over(terminal: &'a File) -> Option<Self> {
        let restore = tcgetattr(terminal).ok()?;
        let mut raw = restore.clone();
        raw.local_modes
            .remove(LocalModes::ECHO | LocalModes::ICANON | LocalModes::ISIG | LocalModes::IEXTEN);
        raw.input_modes.remove(InputModes::IXON);
        // A read that returns after a tenth of a second with nothing, rather
        // than blocking until a key arrives. It is what makes the quiet period
        // the preview waits for observable from inside a blocking read loop.
        raw.special_codes[SpecialCodeIndex::VMIN] = 0;
        raw.special_codes[SpecialCodeIndex::VTIME] = READ_DECISECONDS;
        if tcsetattr(terminal, OptionalActions::Now, &raw).is_err() {
            return None;
        }
        if let Ok(mut saved) = SAVED.lock() {
            *saved = Some(restore.clone());
        }
        Some(Self { terminal, restore })
    }
}

impl Drop for Raw<'_> {
    fn drop(&mut self) {
        let _ = tcsetattr(self.terminal, OptionalActions::Now, &self.restore);
        if let Ok(mut saved) = SAVED.lock() {
            *saved = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DEVICE, Raw};
    use rustix::termios::{InputModes, LocalModes, SpecialCodeIndex, tcgetattr};

    /// The device constant is the device the prompt opens.
    ///
    /// Held by reading `prompt.rs` itself rather than by calling into it: the
    /// claim is that there is one spelling of the path in this crate, and a
    /// second literal is exactly the drift the constant exists to prevent.
    #[test]
    fn the_prompt_opens_the_device_this_module_names() {
        let prompt = include_str!("prompt.rs");
        assert!(
            prompt.contains("tty::DEVICE"),
            "prompt.rs no longer opens the device this module names"
        );
        assert_eq!(
            prompt.matches("\"/dev/tty\"").count(),
            0,
            "prompt.rs carries a second /dev/tty literal beside tty::DEVICE"
        );
        assert_eq!(DEVICE, "/dev/tty");
    }

    /// The guard clears each of the five bits, and puts every one of them back.
    ///
    /// Per bit rather than in aggregate, so that dropping one of them from the
    /// cleared set reddens that bit alone.
    #[test]
    fn raw_mode_clears_five_bits_and_restores_them() {
        use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};

        let Ok(master) = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY) else {
            return;
        };
        assert!(
            grantpt(&master).is_ok(),
            "the pseudoterminal is not granted"
        );
        assert!(
            unlockpt(&master).is_ok(),
            "the pseudoterminal is not unlocked"
        );
        let Ok(name) = ptsname(&master, Vec::new()) else {
            return;
        };
        let Ok(terminal) = std::fs::File::options()
            .read(true)
            .write(true)
            .open(String::from_utf8_lossy(name.as_bytes()).into_owned())
        else {
            return;
        };

        let Ok(before) = tcgetattr(&terminal) else {
            return;
        };
        {
            let guard = Raw::over(&terminal);
            assert!(guard.is_some(), "raw mode was not entered");
            let Ok(during) = tcgetattr(&terminal) else {
                return;
            };
            for bit in [
                LocalModes::ECHO,
                LocalModes::ICANON,
                LocalModes::ISIG,
                LocalModes::IEXTEN,
            ] {
                assert!(
                    !during.local_modes.contains(bit),
                    "raw mode left {bit:?} set"
                );
            }
            assert!(
                !during.input_modes.contains(InputModes::IXON),
                "raw mode left IXON set"
            );
            // The timed read the picker's quiet period is measured with: a read
            // that returns with nothing after a tenth of a second rather than
            // blocking until a key arrives.
            assert_eq!(
                during.special_codes[SpecialCodeIndex::VMIN],
                0,
                "raw mode left a minimum byte count on the read"
            );
            assert_eq!(
                during.special_codes[SpecialCodeIndex::VTIME],
                1,
                "raw mode left the read untimed"
            );
        }

        let Ok(after) = tcgetattr(&terminal) else {
            return;
        };
        assert_eq!(
            after.local_modes, before.local_modes,
            "the local modes were not restored"
        );
        assert_eq!(
            after.input_modes, before.input_modes,
            "the input modes were not restored"
        );
        for code in [SpecialCodeIndex::VMIN, SpecialCodeIndex::VTIME] {
            assert_eq!(
                after.special_codes[code], before.special_codes[code],
                "{code:?} was not restored"
            );
        }
    }
}
