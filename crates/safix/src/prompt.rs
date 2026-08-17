//! Reading a value the operator types, twice, without echoing it.
//!
//! This is the one place a terminal is touched, and it is in the command rather
//! than in the library for that reason. What it produces is a
//! [`Secret`]: the bytes go from the terminal into the
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
//! absent and the two blank lines are present, which is what the integration
//! suite reads and what the differential harness compared before it.

use std::fs::File;
use std::io::{self, Write as _};

use rustix::termios::{LocalModes, OptionalActions, Termios, tcgetattr, tcsetattr};
use safix_core::adduser::Confirm;
use safix_core::enroll::Operator;
use safix_core::enroll::custody::DatabasePassword;
use safix_core::generate::Interaction;
use safix_core::model::PromptKind;
use safix_core::set::ValueSource;
use safix_core::{Error, Result, Secret};

/// The line that ends a multi-line prompt.
const END_OF_INPUT: &str = "EOF";

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
        let mut source = open_source(
            "safix: no terminal; reading the value from stdin (it will not be echoed anyway).",
        );
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

impl Interaction for Prompted {
    fn prompt(&mut self, kind: PromptKind, name: &str, description: &str) -> Result<Secret> {
        let mut source = open_source(&format!(
            "safix: no terminal; reading '{name}' from stdin (it will not be echoed anyway)."
        ));
        let asked = format!("  {name} ({description}): ");
        match kind {
            PromptKind::Hidden => {
                let value =
                    unechoed_line(&mut source, &asked)?.ok_or_else(|| Error::NoValueForPrompt {
                        name: name.to_owned(),
                    })?;
                eprintln!();
                Ok(value)
            }
            PromptKind::Line => {
                echoed_line(&mut source, &asked)?.ok_or_else(|| Error::NoValueForPrompt {
                    name: name.to_owned(),
                })
            }
            PromptKind::Multiline => {
                // Written with `printf` rather than as a `read` prompt, so it
                // appears whether or not the input is a terminal: it is the only
                // place the operator is told how to end the value.
                eprintln!("  {name} ({description}), ending with a line reading {END_OF_INPUT}:");
                match &mut source {
                    Source::Terminal(terminal) => {
                        let mut handle: &File = terminal;
                        Secret::read_until_marker(&mut handle, END_OF_INPUT)
                    }
                    Source::Stdin => {
                        Secret::read_until_marker(&mut io::stdin().lock(), END_OF_INPUT)
                    }
                }
            }
        }
    }

    fn confirm(&mut self, question: &str) -> Result<bool> {
        let mut source = open_source("safix: no terminal; reading the confirmation from stdin.");
        Ok(affirmative(plain_line(&mut source, question).as_deref()))
    }
}

impl Confirm for Prompted {
    /// Standard input rather than a terminal, and the prompt is the caller's:
    /// the shell runtime writes it with `printf` and reads the answer with a
    /// bare `read`, so it appears on a pipe as well as on a terminal.
    fn scaffold(&mut self) -> Result<bool> {
        let mut source = Source::Stdin;
        Ok(affirmative(plain_line(&mut source, "").as_deref()))
    }
}

impl Operator for Prompted {
    /// The PIN of a card safix did not provision, read once and unechoed.
    ///
    /// Once rather than twice, which is the one place this differs from `set`'s
    /// two entries, and the difference is what the value is: `set` reads a value
    /// nothing can check, so the confirmation is the only guard against a typo,
    /// where a PIN the card refuses is caught by the card — at the cost of one of
    /// its three retries, which the refusal names.
    fn card_pin(&mut self, serial: &str) -> Result<Secret> {
        let mut source = open_source(
            "safix: no terminal; reading the card's PIN from stdin (it will not be echoed anyway).",
        );
        eprintln!("safix: {serial} is already provisioned. Its PIN is not echoed.");
        let pin = one_line(&mut source, "  PIN: ")?.ok_or(Error::NoValueRead)?;
        if pin.is_empty() {
            return Err(Error::EmptyValue);
        }
        Ok(pin)
    }
}

impl DatabasePassword for Prompted {
    /// The one password prompt the store's own command path has.
    ///
    /// It is asked here rather than left to `keepassxc-cli` because that command
    /// reads the database password and then the entry password from the same
    /// standard input, and safix has to write the second one — a stream cannot be
    /// both a pipe and a keyboard.
    fn database_password(&mut self, database: &std::path::Path) -> Result<Secret> {
        let mut source = open_source(
            "safix: no terminal; reading the database password from stdin (it will not be \
             echoed anyway).",
        );
        eprintln!(
            "safix: unlocking {} to add the card's access. The password is not echoed.",
            database.display()
        );
        let password = one_line(&mut source, "  password: ")?.ok_or(Error::NoValueRead)?;
        if password.is_empty() {
            return Err(Error::EmptyValue);
        }
        Ok(password)
    }
}

/// Which answers mean yes, which is the shell runtime's own list.
fn affirmative(answer: Option<&str>) -> bool {
    matches!(answer, Some("y" | "Y" | "yes" | "YES"))
}

/// `/dev/tty` when it can be opened, standard input otherwise.
///
/// Probed by opening it for writing, which is what the shell runtime's
/// `{ : >/dev/tty; } 2>/dev/null` does, and then opened again for reading — the
/// two are separate descriptions and the write probe is discarded.
fn open_source(announcement: &str) -> Source {
    if File::options().write(true).open("/dev/tty").is_ok()
        && let Ok(terminal) = File::options().read(true).open("/dev/tty")
    {
        return Source::Terminal(terminal);
    }
    eprintln!("{announcement}");
    Source::Stdin
}

/// One line with the terminal echoing it, which is what a prompt whose answer is
/// not itself a secret is read with.
fn echoed_line(source: &mut Source, prompt: &str) -> Result<Option<Secret>> {
    match source {
        Source::Terminal(terminal) => {
            eprint!("{prompt}");
            let _ = io::stderr().flush();
            let mut handle: &File = terminal;
            Secret::read_line_from(&mut handle)
        }
        Source::Stdin => Secret::read_line_from(&mut io::stdin().lock()),
    }
}

/// One line with the echo off, and no blank line after it.
fn unechoed_line(source: &mut Source, prompt: &str) -> Result<Option<Secret>> {
    match source {
        Source::Terminal(terminal) => {
            eprint!("{prompt}");
            let _ = io::stderr().flush();
            let terminal: &File = terminal;
            let _silenced = Silenced::over(terminal);
            let mut handle = terminal;
            Secret::read_line_from(&mut handle)
        }
        Source::Stdin => Secret::read_line_from(&mut io::stdin().lock()),
    }
}

/// One line that is not a secret, so it may be an ordinary string.
///
/// A confirmation is an answer to a question this program asked, not a value it
/// is about to store, and reading it as a [`Secret`] would put it through a type
/// whose whole point is that it cannot be compared against a literal.
fn plain_line(source: &mut Source, prompt: &str) -> Option<String> {
    let mut line = Vec::new();
    let mut byte = [0_u8; 1];
    let read: &mut dyn io::Read = match source {
        Source::Terminal(terminal) => {
            eprint!("{prompt}");
            let _ = io::stderr().flush();
            terminal
        }
        Source::Stdin => &mut io::stdin().lock(),
    };
    loop {
        match read.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) if byte.first() == Some(&b'\n') => {
                return String::from_utf8(line).ok();
            }
            Ok(_) => line.extend_from_slice(&byte),
            Err(cause) if cause.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
}

/// One line, prompted and unechoed on a terminal, silent on anything else.
///
/// The blank line after it is written whether or not there was a prompt, and
/// only when a line arrived. The shell runtime spells that as
/// `read ... || die "no value read"` with the `printf` on the line after, so a
/// stream that ended mid-value ends the run before the newline is reached.
fn one_line(source: &mut Source, prompt: &str) -> Result<Option<Secret>> {
    // `&File` is the reader inside `unechoed_line`, so the guard's borrow of the
    // same terminal stays immutable and the restore still runs after the read.
    let read = unechoed_line(source, prompt);
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
