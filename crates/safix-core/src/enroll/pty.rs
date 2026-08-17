//! One command on a pseudo-terminal, answered once.
//!
//! `age-plugin-yubikey --generate` reads the PIN from a terminal and from
//! nowhere else. There is no flag for it, and its prompt is `dialoguer`'s, which
//! returns the empty string when the stream it would prompt on is not a
//! terminal. A pseudo-terminal is therefore not a convenience here; it is the
//! only programmatic path, and this module is the whole of it.
//!
//! # How the prompt is recognised
//!
//! By shape, not by text. A password prompt is a program turning the terminal's
//! echo off, and the pseudo-terminal's attributes are readable from the master
//! end, so the falling edge of `ECHO` is the prompt — whatever wording the
//! plugin puts in front of it, in whatever language its translations are loaded
//! in. A plugin upgrade that rewords the prompt still gets the PIN.
//!
//! # One attempt, and why
//!
//! A wrong PIN costs a retry, and a card has three. So the answer is written on
//! the first falling edge and never again: a second prompt means the first
//! answer was rejected, and the run stops there with the counter at two rather
//! than walking it to zero. The refusal says so.
//!
//! # Which stream gets the terminal
//!
//! Standard input and standard error, and not standard output. That split is
//! load-bearing in both directions. `dialoguer` writes its prompt to standard
//! error and reads the answer from standard input, and it gives up without
//! reading when standard error is not a terminal — so both of those must be the
//! pseudo-terminal. The plugin also asks whether standard *output* is a terminal
//! and prints the recipient to standard error when it is not, and the identity
//! block it prints is a document rather than commentary — so standard output is
//! a pipe, which keeps the block out of the interleaved prompt stream and makes
//! it readable as bytes.
//!
//! # What the operator sees
//!
//! Everything on the terminal side, as it arrives, including the instruction to
//! touch the card. That is the point of draining the master while the child
//! runs rather than after it: a touch instruction delivered after the touch has
//! timed out is not an instruction.

use std::io::{Read as _, Write as _};
use std::os::fd::{AsFd as _, OwnedFd};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
use rustix::termios::{LocalModes, tcgetattr};

use crate::error::{Error, Result};
use crate::progress::Progress;
use crate::secret::Secret;

/// How long the run waits with nothing happening before giving up.
///
/// Generous, because the interaction it covers is a person finding a card and
/// touching it. It exists so that a child which has stopped talking and will
/// never talk again ends the run rather than holding a terminal forever.
pub const DEFAULT_IDLE_LIMIT: Duration = Duration::from_secs(90);

/// How long each turn of the drain loop sleeps when the master has nothing.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// What a run on a pseudo-terminal produced.
///
/// No `Debug`: `stdout` is a document the plugin printed and `terminal` is a
/// transcript of a session in which a PIN was typed, and while neither should
/// carry the PIN itself — the echo was off — a derived rendering of both is a
/// rendering of a credential exchange.
pub struct Session {
    /// Everything the child wrote to its standard output, which is a pipe.
    pub stdout: Vec<u8>,
    /// Everything that crossed the terminal, in arrival order.
    pub terminal: String,
    /// What the child exited with, or one when a signal ended it.
    pub status: i32,
    /// Whether a password prompt was seen and answered.
    pub answered: bool,
}

/// Run one command with a pseudo-terminal on its input and its commentary,
/// answering the first password prompt with `answer` and nothing after it.
///
/// # Errors
///
/// [`Error::PtyUnusable`] when a pseudo-terminal cannot be opened or read,
/// [`Error::PluginUnavailable`] when the command cannot be run,
/// [`Error::CardPinRejected`] when a second prompt arrives after the answer was
/// given, and [`Error::PluginFailed`] when the run stalls past `idle_limit`.
pub fn answering_once(
    command: &mut Command,
    answer: &Secret,
    serial: &str,
    progress: &dyn Progress,
    idle_limit: Duration,
) -> Result<Session> {
    let pair = Pair::open()?;

    command
        .stdin(Stdio::from(pair.slave_copy()?))
        .stdout(Stdio::piped())
        .stderr(Stdio::from(pair.slave_copy()?));

    let mut child = command.spawn().map_err(|cause| Error::PluginUnavailable {
        program: program_of(command),
        cause,
    })?;

    // Dropped before the drain begins, so the only descriptions of the slave
    // left open belong to the child: a read of the master then reports the
    // child's exit rather than blocking on a description this process is holding.
    drop(pair.slave);

    let outcome = drain(
        &pair.master,
        &mut child,
        answer,
        serial,
        progress,
        idle_limit,
    );

    let stdout = {
        let mut collected = Vec::new();
        if let Some(mut pipe) = child.stdout.take() {
            let _ = pipe.read_to_end(&mut collected);
        }
        collected
    };

    let status = child.wait().map_or(1, |status| status.code().unwrap_or(1));

    let drained = outcome?;
    Ok(Session {
        stdout,
        terminal: drained.text,
        status,
        answered: drained.answered,
    })
}

/// What the drain loop saw.
struct Drained {
    text: String,
    answered: bool,
}

/// Read the master until the child is done, answering the first prompt.
fn drain(
    master: &OwnedFd,
    child: &mut Child,
    answer: &Secret,
    serial: &str,
    progress: &dyn Progress,
    idle_limit: Duration,
) -> Result<Drained> {
    let mut text = String::new();
    let mut answered: Option<Instant> = None;
    let mut last_movement = Instant::now();
    let mut buffer = [0_u8; 4096];

    loop {
        // Sampled every turn rather than only when the master has nothing, because
        // a prompt is a state and not an event: the echo goes off, is restored the
        // instant the line is read, and goes off again for a second prompt, all
        // faster than any polling interval. A watcher looking for the transition
        // would miss the restoration in between and then see no transition at all.
        match asked(master, answered, idle_limit) {
            Asked::No => (),
            Asked::First => {
                write_answer(master, answer)?;
                answered = Some(Instant::now());
                last_movement = Instant::now();
            }
            Asked::Again => {
                let _ = child.kill();
                return Err(Error::CardPinRejected {
                    serial: serial.to_owned(),
                });
            }
        }

        match rustix::io::read(master, &mut buffer) {
            Ok(read) if read > 0 => {
                last_movement = Instant::now();
                if let Some(bytes) = buffer.get(..read) {
                    let shown = String::from_utf8_lossy(bytes);
                    progress.write(&shown);
                    text.push_str(&shown);
                }
            }
            // Zero bytes is end of stream, and `IO` is what a Linux master reports
            // once no description of the slave is left. Either way there is nothing
            // more to drain.
            Ok(_) | Err(rustix::io::Errno::IO) => break,
            // A signal arrived mid-read; the next turn reads again.
            Err(rustix::io::Errno::INTR) => (),
            Err(rustix::io::Errno::AGAIN) => {
                // The child's exit is what ends the drain, and it is asked about
                // rather than inferred from the master reporting `IO`. That report
                // only comes once every description of the slave is closed, and
                // this process holds two of them for as long as the `Command` that
                // was spawned is alive — `spawn` borrows it rather than consuming
                // it, so the streams it was configured with outlive the run. With
                // nothing queued and the child gone there is nothing further to
                // read, which is exactly this branch plus that answer.
                if child.try_wait().ok().flatten().is_some() {
                    break;
                }
                if last_movement.elapsed() >= idle_limit {
                    let _ = child.kill();
                    return Err(Error::PluginStalled {
                        seconds: idle_limit.as_secs(),
                    });
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(cause) => {
                let _ = child.kill();
                return Err(Error::PtyUnusable {
                    cause: std::io::Error::from(cause),
                });
            }
        }
    }

    Ok(Drained {
        text,
        answered: answered.is_some(),
    })
}

/// What the echo state says about whether a password is being asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asked {
    /// Nothing is waiting on a password right now.
    No,
    /// A prompt is up and has not been answered.
    First,
    /// A prompt is up and one was already answered, so the answer was rejected.
    Again,
}

/// How long after answering a prompt an echo that is still off is a new prompt.
///
/// A password read restores the terminal the instant the line arrives — the
/// answer is written with its newline, so the read returns immediately — which
/// makes an echo still off well after that a second prompt rather than the tail of
/// the first. Generous enough that no plausible restoration is mistaken for a
/// re-prompt, and short enough that the run does not spend a card's retries
/// waiting to be sure.
const REPROMPT_GRACE: Duration = Duration::from_millis(250);

/// Whether a password prompt is up, and whether it is the second one.
fn asked(master: &OwnedFd, answered: Option<Instant>, idle_limit: Duration) -> Asked {
    if !echo_is_off(master) {
        return Asked::No;
    }
    match answered {
        None => Asked::First,
        Some(at) if at.elapsed() >= REPROMPT_GRACE.min(idle_limit) => Asked::Again,
        Some(_) => Asked::No,
    }
}

/// Whether the pseudo-terminal currently has echo turned off.
///
/// A terminal whose attributes cannot be read is reported as echoing, which is
/// the safe answer: the run then waits rather than writing a PIN at a moment
/// nothing asked for one.
fn echo_is_off(master: &OwnedFd) -> bool {
    tcgetattr(master.as_fd())
        .is_ok_and(|attributes| !attributes.local_modes.contains(LocalModes::ECHO))
}

/// The answer and the newline that submits it, down the terminal.
fn write_answer(master: &OwnedFd, answer: &Secret) -> Result<()> {
    let mut sink = std::fs::File::from(
        master
            .try_clone()
            .map_err(|cause| Error::PtyUnusable { cause })?,
    );
    answer
        .write_to(&mut sink)
        .and_then(|()| sink.write_all(b"\n"))
        .and_then(|()| sink.flush())
        .map_err(|cause| Error::PtyUnusable { cause })
}

/// A pseudo-terminal pair, master held here and slave handed to the child.
struct Pair {
    master: OwnedFd,
    slave: OwnedFd,
}

impl Pair {
    /// Open one pair, with the master non-blocking so the drain loop can look at
    /// the terminal's attributes between reads.
    fn open() -> Result<Self> {
        use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};

        let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY).map_err(unusable)?;
        // Non-blocking after the fact rather than at open: `OpenptFlags` has no
        // bit for it, and the drain loop has to be able to look at the terminal's
        // attributes between reads rather than block inside one.
        let flags = fcntl_getfl(&master).map_err(unusable)?;
        fcntl_setfl(&master, flags | OFlags::NONBLOCK).map_err(unusable)?;
        grantpt(&master).map_err(unusable)?;
        unlockpt(&master).map_err(unusable)?;

        let name = ptsname(&master, Vec::new()).map_err(unusable)?;
        let path = std::path::PathBuf::from(String::from_utf8_lossy(name.as_bytes()).into_owned());
        let slave = std::fs::File::options()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|cause| Error::PtyUnusable { cause })?;

        Ok(Self {
            master,
            slave: OwnedFd::from(slave),
        })
    }

    /// A second description of the slave, for one of the child's streams.
    fn slave_copy(&self) -> Result<OwnedFd> {
        self.slave
            .try_clone()
            .map_err(|cause| Error::PtyUnusable { cause })
    }
}

fn unusable(cause: rustix::io::Errno) -> Error {
    Error::PtyUnusable {
        cause: std::io::Error::from(cause),
    }
}

/// The program a command will run, for a refusal that has to name it.
fn program_of(command: &Command) -> String {
    command.get_program().to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::progress::Recorded;

    /// A shell that turns echo off, prompts, reads a line and reports it.
    ///
    /// `stty -echo` is the shape the wrapper answers on, spelled by the tool
    /// every unix has rather than by a program this suite would have to build,
    /// so the claim is about the shape and not about a stand-in that agrees with
    /// it.
    fn asking(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.arg("-c").arg(script);
        command
    }

    fn secret(text: &str) -> Secret {
        Secret::read_from(&mut Cursor::new(text.as_bytes().to_vec())).expect("a cursor can be read")
    }

    #[test]
    fn the_prompt_is_answered_once_and_the_answer_reaches_the_command() {
        let recorded = Recorded::default();
        let session = answering_once(
            &mut asking(
                "printf 'about to ask\\n' >&2; \
                 stty -echo; printf 'PIN: ' >&2; read -r answer; stty echo; \
                 printf '\\n' >&2; printf 'got=%s\\n' \"$answer\"",
            ),
            &secret("87654321"),
            "12345678",
            &recorded,
            Duration::from_secs(20),
        )
        .expect("the wrapper answers a prompt");

        assert_eq!(session.status, 0);
        assert!(session.answered, "no prompt was recognised");
        assert_eq!(
            String::from_utf8_lossy(&session.stdout).trim_end(),
            "got=87654321",
            "the answer did not reach the command"
        );
        assert!(
            session.terminal.contains("about to ask"),
            "the command's commentary did not cross the terminal"
        );
        assert!(
            recorded.written().contains("PIN: "),
            "the prompt did not reach the operator"
        );
    }

    #[test]
    fn a_second_prompt_aborts_rather_than_spending_another_retry() {
        let recorded = Recorded::default();
        let refusal = answering_once(
            &mut asking(
                "stty -echo; printf 'PIN: ' >&2; read -r one; stty echo; printf '\\n' >&2; \
                 stty -echo; printf 'PIN again: ' >&2; read -r two; stty echo; \
                 printf 'never reached\\n'",
            ),
            &secret("87654321"),
            "12345678",
            &recorded,
            Duration::from_secs(20),
        );

        assert!(
            matches!(
                refusal,
                Err(Error::CardPinRejected { ref serial }) if serial == "12345678"
            ),
            "a second prompt is not a rejected first answer"
        );
    }

    #[test]
    fn a_command_that_never_asks_runs_to_completion_unanswered() {
        let recorded = Recorded::default();
        let session = answering_once(
            &mut asking("printf 'no prompt here\\n' >&2; printf 'done\\n'"),
            &secret("87654321"),
            "12345678",
            &recorded,
            Duration::from_secs(20),
        )
        .expect("a command that asks nothing is not a failure");

        assert_eq!(session.status, 0);
        assert!(!session.answered);
        assert_eq!(String::from_utf8_lossy(&session.stdout).trim_end(), "done");
    }

    #[test]
    fn standard_output_is_a_pipe_and_standard_error_is_the_terminal() {
        let recorded = Recorded::default();
        let session = answering_once(
            &mut asking(
                "if [ -t 1 ]; then printf 'stdout=terminal\\n'; else printf 'stdout=pipe\\n'; fi; \
                 if [ -t 2 ]; then printf 'stderr=terminal\\n'; else printf 'stderr=pipe\\n'; fi; \
                 if [ -t 0 ]; then printf 'stdin=terminal\\n'; else printf 'stdin=pipe\\n'; fi",
            ),
            &secret("87654321"),
            "12345678",
            &recorded,
            Duration::from_secs(20),
        )
        .expect("a command that only asks questions of its streams");

        let reported = String::from_utf8_lossy(&session.stdout).into_owned();
        assert!(reported.contains("stdout=pipe"), "{reported}");
        assert!(reported.contains("stderr=terminal"), "{reported}");
        assert!(reported.contains("stdin=terminal"), "{reported}");
    }

    #[test]
    fn a_command_that_stalls_ends_the_run_rather_than_holding_the_terminal() {
        let recorded = Recorded::default();
        let refusal = answering_once(
            &mut asking("read -r nothing"),
            &secret("87654321"),
            "12345678",
            &recorded,
            Duration::from_millis(300),
        );

        assert!(
            matches!(refusal, Err(Error::PluginStalled { .. })),
            "a command that waits forever was not refused"
        );
    }

    #[test]
    fn a_command_that_cannot_be_run_is_refused_by_name() {
        let recorded = Recorded::default();
        let refusal = answering_once(
            &mut Command::new("safix-no-such-plugin-command"),
            &secret("87654321"),
            "12345678",
            &recorded,
            Duration::from_secs(1),
        );

        assert!(
            matches!(refusal, Err(Error::PluginUnavailable { .. })),
            "a program that does not exist was not refused by name"
        );
    }
}
