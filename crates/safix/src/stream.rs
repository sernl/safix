//! Reading a value a program piped, rather than one a person typed.
//!
//! The other half of [`crate::prompt`], and the reason both are here rather than
//! in the library: [`safix_core::set`] is terminal-free and takes the value
//! through a [`ValueSource`], so where the value comes from is the command's to
//! decide. This is the source for a `set` nobody is watching — a script, a
//! pipeline, an enrolment tool — and it is the contract safix's own bridge
//! already relies on when it writes into clan.
//!
//! # What it does not do, and why that is the point
//!
//! No prompt, because there is nobody to prompt. No confirmation either: the
//! double prompt exists to catch a value mistyped invisibly, and a piped value
//! has no typist. Removing a human checkpoint where no human is present is not a
//! concession — the checks a pipe can actually have are the two below, and it
//! keeps both.
//!
//! Bytes are stored exactly as received, which is the doctrine the generator
//! contract already states: `echo` pipes a trailing newline and `printf` does
//! not, and nothing here removes one. And an empty stream takes the same refusal
//! an empty prompt takes, because an empty pipe is the state a failed upstream
//! command leaves behind — the one mistake a script makes that a person does not.

use std::io;

use safix_core::set::ValueSource;
use safix_core::{Error, Result, Secret};

/// Reads the value from this process's standard input, to end of stream.
pub struct Piped;

impl ValueSource for Piped {
    /// The whole of standard input, refusing nothing but emptiness.
    ///
    /// The user and the name are unused: a stream carries one value and needs no
    /// address, where a prompt has to say whose secret it is asking for.
    fn read(&mut self, _user: &str, _name: &str) -> Result<Secret> {
        let value = Secret::read_from_stdin()?;
        if value.is_empty() {
            return Err(Error::EmptyValue);
        }
        Ok(value)
    }
}

/// Whether a person is at the other end of standard input.
///
/// The terminal test on standard input, which is exactly the branch `clan vars
/// set` takes, so that one piece of calling code scripts both commands. It is
/// asked of standard input rather than of `/dev/tty`: a run whose standard input
/// is redirected is a run nobody is typing into, whether or not the machine
/// happens to have a terminal somewhere.
#[must_use]
pub fn stdin_is_a_terminal() -> bool {
    io::IsTerminal::is_terminal(&io::stdin())
}
