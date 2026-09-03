//! One command on a pseudo-terminal, answered prompt by prompt.
//!
//! Two tools on this path read a credential from a terminal and from nowhere
//! else, and neither has a flag that takes one off a pipe.
//! `age-plugin-yubikey --generate` prompts through `dialoguer`, which returns the
//! empty string when the stream it would prompt on is not a terminal. `ykman piv
//! access` prompts through `click` when its credential options are omitted — and
//! omitting them is the point, because the alternative is a PIN in an argument
//! vector, which is a channel any process on the machine can read. A
//! pseudo-terminal is therefore not a convenience here; it is the only channel
//! that is both programmatic and private, and this module is the whole of it.
//!
//! # How a prompt is recognised
//!
//! By shape, not by text. A password prompt is a program turning the terminal's
//! echo off, and the pseudo-terminal's attributes are readable from the master
//! end, so an echo that is off is a prompt — whatever wording is in front of it,
//! in whatever language the tool's translations are loaded in. A tool upgrade
//! that rewords a prompt still gets its answer.
//!
//! # One value, a bounded number of times
//!
//! Every prompt of one invocation gets the same value, and at most `limit` of
//! them are answered. That is deliberately not a sequence of different answers,
//! and the reason is that a sequence cannot be paced soundly. Both tools set and
//! restore the terminal with `TCSAFLUSH`, which discards input that has arrived
//! and not been read, so answers cannot be written ahead of the prompts they
//! belong to; and nothing observable separates one prompt from the next, because
//! a hidden read restores the terminal the instant the answer arrives and the
//! following read turns it off again, both far faster than any polling interval.
//! A wrapper that guessed at those boundaries would put the wrong value in the
//! wrong prompt, occasionally, under load.
//!
//! Every prompt this drives asks for the same thing, so the boundaries do not
//! have to be found. The generator asks for the PIN once. `change-management-key
//! --protect` asks for the PIN once. `change-pin` and `change-puk` ask for the new
//! credential and then for it again as a confirmation — one value, twice — because
//! the *current* credential is supplied as a flag, and on the only card this ever
//! runs against that current credential is the published factory default rather
//! than anything safix generated.
//!
//! # Where a retry is actually spent, and what the bound protects
//!
//! A retry is spent by submitting a wrong value to the card, not by being
//! prompted and not by declining to answer. In `change-pin` and `change-puk` the
//! submitted-to-the-card value is the *current* one, which the tool takes from
//! its flag and submits exactly once; the prompted value is the new credential,
//! which no counter judges. In `change-management-key --protect` and in the
//! generator the prompted value *is* the submitted one, and there the bound is
//! one: a second prompt means the card refused what it was given, and the run
//! stops with the counter one below where it started rather than walking it to
//! zero.
//!
//! A prompt arriving past the bound is not answered at all, which is also what
//! covers a tool that asks more questions than the caller expected: the run
//! aborts having submitted only what it was told to, which for a provisioning
//! drive means a card that was not provisioned rather than a card whose retries
//! were spent finding out.
//!
//! # Why the wrapper waits for quiet before answering
//!
//! A prompt's own text is written *after* the echo goes off — `getpass` and
//! `console` both set the terminal first and print second — so an echo that has
//! just gone off is not yet a prompt that is waiting. Answering it immediately
//! would sometimes put a second copy of the value in the queue behind the first.
//! So the wrapper answers only once the terminal has been quiet for a polling
//! interval and its own last answer is at least that old. Inside a prompt the
//! tool already holds, quiet never lasts that long: it has the line and moves on
//! in microseconds. Waiting for genuine silence is therefore the difference
//! between a prompt that is waiting and one that is still being written.
//!
//! Two more conditions separate a prompt from a tool that is merely slow. The
//! wrapper judges the terminal only when the master has nothing queued: a tool
//! that prints the newline closing one hidden read, restores the terminal
//! through a subprocess, and only then writes its next prompt hands the wrapper
//! those bytes in separate reads under load, and a judgement made between them
//! would take the newline, the dark terminal, and the gap for a prompt. And a
//! starved tool, or one that restores the terminal slowly, can hold the echo
//! off for longer than a polling interval after the answer has been written,
//! during which quiet plus a dark terminal looks exactly like a second prompt.
//! A second prompt, though, is always *written*: the refusal and the prompt
//! text arrive before it waits. So after an answer the wrapper also requires
//! that the child has written something since, and a tool that is only slow to
//! read is left alone rather than judged to have asked again.
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
    /// How many prompts were seen and answered.
    pub answered: usize,
}

/// Run one command with a pseudo-terminal on its input and its commentary,
/// answering up to `limit` password prompts with `answer` and none beyond them.
///
/// # Errors
///
/// [`Error::PtyUnusable`] when a pseudo-terminal cannot be opened or read,
/// [`Error::PluginUnavailable`] when the command cannot be run,
/// [`Error::CardPinRejected`] when a prompt arrives past `limit`, and
/// [`Error::PluginStalled`] when the run says nothing for `idle_limit`.
pub fn answering(
    command: &mut Command,
    answer: &Secret,
    limit: usize,
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
        limit,
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
    answered: usize,
}

/// Read the master until the child is done, answering the prompts it makes.
fn drain(
    master: &OwnedFd,
    child: &mut Child,
    answer: &Secret,
    limit: usize,
    serial: &str,
    progress: &dyn Progress,
    idle_limit: Duration,
) -> Result<Drained> {
    let mut text = String::new();
    let mut answered: usize = 0;
    let mut last_byte = Instant::now();
    let mut last_answer: Option<Instant> = None;
    let mut last_movement = Instant::now();
    let mut buffer = [0_u8; 4096];

    loop {
        match rustix::io::read(master, &mut buffer) {
            Ok(read) if read > 0 => {
                last_byte = Instant::now();
                last_movement = last_byte;
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
                // A prompt that is waiting, rather than one still being written:
                // nothing is queued on the master, the echo is off, nothing has
                // arrived for a polling interval, this wrapper's own last answer
                // is at least that old, and the child has written something since
                // that answer. The module documentation states why each of the
                // five is load-bearing.
                let waiting = echo_is_off(master)
                    && last_byte.elapsed() >= POLL_INTERVAL
                    && last_answer.is_none_or(|at| at.elapsed() >= POLL_INTERVAL && last_byte > at);
                if waiting {
                    if answered >= limit {
                        // Not answered, and that is the whole of the protection: a
                        // retry is spent by submitting a value, so declining to
                        // submit one costs nothing and stops the run one below
                        // where it started.
                        let _ = child.kill();
                        return Err(Error::CardPinRejected {
                            serial: serial.to_owned(),
                        });
                    }
                    write_answer(master, answer)?;
                    answered = answered.saturating_add(1);
                    last_answer = Some(Instant::now());
                    last_movement = Instant::now();
                    continue;
                }
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

    Ok(Drained { text, answered })
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
        let session = answering(
            &mut asking(
                "printf 'about to ask\\n' >&2; \
                 stty -echo; printf 'PIN: ' >&2; read -r answer; stty echo; \
                 printf '\\n' >&2; printf 'got=%s\\n' \"$answer\"",
            ),
            &secret("87654321"),
            1,
            "12345678",
            &recorded,
            Duration::from_secs(20),
        )
        .expect("the wrapper answers a prompt");

        assert_eq!(session.status, 0);
        assert_eq!(session.answered, 1, "the one prompt was not answered once");
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

    /// A value and its confirmation, which is the shape `change-pin` asks in.
    ///
    /// Two prompts, one value, and the terminal set and restored around each read
    /// the way `click` does it — so the run has no observable gap between one
    /// prompt and the next, which is the case this wrapper is built not to need.
    #[test]
    fn a_value_and_its_confirmation_are_both_answered_with_the_one_value() {
        let recorded = Recorded::default();
        let session = answering(
            &mut asking(
                "for label in new again; do \
                   stty -echo; printf '%s: ' \"$label\" >&2; read -r answer; stty echo; \
                   printf '\\n' >&2; printf '%s=%s\\n' \"$label\" \"$answer\"; \
                 done",
            ),
            &secret("22222222"),
            2,
            "12345678",
            &recorded,
            Duration::from_secs(20),
        )
        .expect("the wrapper answers a value and its confirmation");

        assert_eq!(session.status, 0);
        assert_eq!(session.answered, 2, "not every prompt was answered");
        assert_eq!(
            String::from_utf8_lossy(&session.stdout).trim_end(),
            "new=22222222\nagain=22222222",
            "the value did not reach both prompts"
        );
    }

    /// A tool that asks one more question than the bound allows is not answered.
    #[test]
    fn a_prompt_past_the_bound_aborts_with_nothing_further_submitted() {
        let recorded = Recorded::default();
        let refusal = answering(
            &mut asking(
                "for label in one two; do \
                   stty -echo; printf '%s: ' \"$label\" >&2; read -r answer; stty echo; \
                   printf '\\n' >&2; printf '%s=%s\\n' \"$label\" \"$answer\"; \
                 done; printf 'never reached\\n'",
            ),
            &secret("111111"),
            1,
            "12345678",
            &recorded,
            Duration::from_secs(20),
        );

        assert!(
            matches!(
                refusal,
                Err(Error::CardPinRejected { ref serial }) if serial == "12345678"
            ),
            "a prompt past the bound was answered anyway"
        );
    }

    #[test]
    fn a_second_prompt_aborts_rather_than_spending_another_retry() {
        let recorded = Recorded::default();
        let refusal = answering(
            &mut asking(
                "stty -echo; printf 'PIN: ' >&2; read -r one; stty echo; printf '\\n' >&2; \
                 stty -echo; printf 'PIN again: ' >&2; read -r two; stty echo; \
                 printf 'never reached\\n'",
            ),
            &secret("87654321"),
            1,
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
        let session = answering(
            &mut asking("printf 'no prompt here\\n' >&2; printf 'done\\n'"),
            &secret("87654321"),
            1,
            "12345678",
            &recorded,
            Duration::from_secs(20),
        )
        .expect("a command that asks nothing is not a failure");

        assert_eq!(session.status, 0);
        assert_eq!(session.answered, 0);
        assert_eq!(String::from_utf8_lossy(&session.stdout).trim_end(), "done");
    }

    #[test]
    fn standard_output_is_a_pipe_and_standard_error_is_the_terminal() {
        let recorded = Recorded::default();
        let session = answering(
            &mut asking(
                "if [ -t 1 ]; then printf 'stdout=terminal\\n'; else printf 'stdout=pipe\\n'; fi; \
                 if [ -t 2 ]; then printf 'stderr=terminal\\n'; else printf 'stderr=pipe\\n'; fi; \
                 if [ -t 0 ]; then printf 'stdin=terminal\\n'; else printf 'stdin=pipe\\n'; fi",
            ),
            &secret("87654321"),
            1,
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
        let refusal = answering(
            &mut asking("read -r nothing"),
            &secret("87654321"),
            1,
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
        let refusal = answering(
            &mut Command::new("safix-no-such-plugin-command"),
            &secret("87654321"),
            1,
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
