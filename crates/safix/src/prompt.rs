//! Reading a value the operator types, twice, without echoing it.
//!
//! This is the one place a terminal is touched, and it is in the command rather
//! than in the library for that reason. What it produces is a
//! [`Secret`](safix_core::Secret): the bytes go from the terminal into the
//! buffer that type owns and out of it only down a pipe, so there is no
//! intermediate `String` for a panic message or a log line to pick up.
//!
//! # Where the value is read from
//!
//! `/dev/tty` when it opens, this process's standard input when it does not.
//! The terminal is preferred so that a redirected standard input does not
//! silently swallow the prompt; when there is none — a hermetic check, a
//! pipeline — the same reads run against standard input and say so, rather than
//! failing on a machine with no controlling terminal.
//!
//! # Why the prompt is conditional and the blank line is not
//!
//! `bash`'s `read -p` writes its prompt only when the input is a terminal, and
//! writes the newline after it unconditionally, because the script writes that
//! newline itself. Both halves are reproduced: on a pipe the two prompts are
//! absent and the two blank lines are present, which is what the differential
//! harness sees.

use std::fs::File;
use std::io::{self, Write as _};

use rustix::termios::{LocalModes, OptionalActions, Termios, tcgetattr, tcsetattr};
use safix_core::set::ValueSource;
use safix_core::{Error, Result, Secret};

/// The terminal this process can reach, or the absence of one.
enum Source {
    /// `/dev/tty`, opened for reading. Echo is turned off while a value is being
    /// read and restored however the read ends.
    Terminal(File),
    /// This process's standard input, which echoes nothing of its own.
    Stdin,
}

/// Reads the value twice from a terminal, or from standard input when there is
/// none.
pub struct Prompted;

impl ValueSource for Prompted {
    fn read(&mut self, user: &str, name: &str) -> Result<Secret> {
        let mut source = open_source();
        eprintln!("safix: setting {name} for {user}. The value is not echoed.");

        let value = one_line(&mut source, "  value: ")?.ok_or(Error::NoValueRead)?;
        let again = one_line(&mut source, "  again: ")?.ok_or(Error::NoConfirmationRead)?;

        if value.is_empty() {
            return Err(Error::EmptyValue);
        }
        if !value.equals(&again) {
            return Err(Error::EntriesDiffer);
        }
        Ok(value)
    }
}

/// `/dev/tty` when it can be opened, standard input otherwise.
///
/// Probed by opening it for writing, which is what the shell runtime's
/// `{ : >/dev/tty; } 2>/dev/null` does, and then opened again for reading — the
/// two are separate descriptions and the write probe is discarded.
fn open_source() -> Source {
    if File::options().write(true).open("/dev/tty").is_ok()
        && let Ok(terminal) = File::options().read(true).open("/dev/tty")
    {
        return Source::Terminal(terminal);
    }
    eprintln!("safix: no terminal; reading the value from stdin (it will not be echoed anyway).");
    Source::Stdin
}

/// One line, prompted and unechoed on a terminal, silent on anything else.
///
/// The blank line after it is written whether or not there was a prompt, and
/// only when a line arrived. The shell runtime spells that as
/// `read ... || die "no value read"` with the `printf` on the line after, so a
/// stream that ended mid-value ends the run before the newline is reached.
fn one_line(source: &mut Source, prompt: &str) -> Result<Option<Secret>> {
    let read = match source {
        Source::Terminal(terminal) => {
            eprint!("{prompt}");
            let _ = io::stderr().flush();
            let terminal: &File = terminal;
            let _silenced = Silenced::over(terminal);
            // `&File` is the reader, so the guard's borrow of the same terminal
            // stays immutable and the restore still runs after the read.
            let mut handle = terminal;
            Secret::read_line_from(&mut handle)
        }
        Source::Stdin => Secret::read_line_from(&mut io::stdin().lock()),
    };
    if matches!(read, Ok(Some(_))) {
        eprintln!();
    }
    read
}

/// Echo turned off for as long as this lives.
///
/// Restored on the way out however the read ended, including through a panic; a
/// terminal left with echo off is one an operator has to type `stty sane` into
/// blind. A terminal whose attributes cannot be read is left alone and the value
/// is read anyway — refusing to read would be a worse answer than reading with
/// the echo the operator can see.
struct Silenced<'a> {
    terminal: &'a File,
    restore: Option<Termios>,
}

impl<'a> Silenced<'a> {
    fn over(terminal: &'a File) -> Self {
        let restore = tcgetattr(terminal).ok();
        if let Some(attributes) = restore.clone() {
            let mut silent = attributes;
            silent.local_modes.remove(LocalModes::ECHO);
            let _ = tcsetattr(terminal, OptionalActions::Now, &silent);
        }
        Self { terminal, restore }
    }
}

impl Drop for Silenced<'_> {
    fn drop(&mut self) {
        if let Some(attributes) = &self.restore {
            let _ = tcsetattr(self.terminal, OptionalActions::Now, attributes);
        }
    }
}
