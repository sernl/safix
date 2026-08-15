//! Where a run's running commentary goes.
//!
//! The read paths answer a question and return the answer; the write paths do
//! work and say what they are doing while they do it, and some of it has to be
//! said before the operator is asked for a value. That ordering is why this is a
//! sink handed in rather than a transcript handed back.
//!
//! The text is built here rather than at the sink, because it is the shell
//! runtime's prose and the differential harness compares it byte for byte. A
//! sink that formatted would be a second author of a tested string.

use std::sync::Mutex;

/// A sink for the text a run writes as it goes.
///
/// Verbatim: the newlines, the indentation and the blank lines are part of the
/// message, so an implementation writes what it is given and adds nothing.
pub trait Progress {
    /// Write this text, exactly as given.
    ///
    /// The commentary channel, which for the command is standard error.
    fn write(&self, text: &str);

    /// Write bytes a subprocess produced on its own standard output.
    ///
    /// A separate channel because it is a separate channel: when a re-wrap is
    /// run with its streams captured rather than inherited, whatever sops wrote
    /// to standard output has to reach standard output and not be folded into
    /// the commentary. Bytes rather than text, because they are not ours to
    /// re-encode.
    fn write_output(&self, bytes: &[u8]);
}

/// A [`Progress`] that discards everything.
///
/// What an embedder uses when it wants the effect and neither the commentary
/// nor a subprocess's output.
#[derive(Debug, Clone, Copy, Default)]
pub struct Silent;

impl Progress for Silent {
    fn write(&self, _text: &str) {}
    fn write_output(&self, _bytes: &[u8]) {}
}

/// A [`Progress`] that keeps what it was given, for tests.
///
/// The two channels are kept apart, because a test that could not tell them
/// apart could not catch output moving between them.
#[derive(Default)]
pub struct Recorded {
    commentary: Mutex<String>,
    output: Mutex<Vec<u8>>,
}

impl Recorded {
    /// Everything written to the commentary channel, in order.
    #[must_use]
    pub fn written(&self) -> String {
        self.commentary.lock().map_or_else(
            |poisoned| poisoned.into_inner().clone(),
            |text| text.clone(),
        )
    }

    /// Everything written to the output channel, in order.
    #[must_use]
    pub fn output(&self) -> Vec<u8> {
        self.output.lock().map_or_else(
            |poisoned| poisoned.into_inner().clone(),
            |bytes| bytes.clone(),
        )
    }
}

impl Progress for Recorded {
    fn write(&self, text: &str) {
        let mut held = self
            .commentary
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.push_str(text);
    }

    fn write_output(&self, bytes: &[u8]) {
        let mut held = self
            .output
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.extend_from_slice(bytes);
    }
}

/// One line, as the shell runtime's `log` writes it.
pub(crate) fn log(progress: &dyn Progress, line: &str) {
    progress.write(&format!("{line}\n"));
}

/// One line indented by two spaces, as the shell runtime's `note` writes it.
pub(crate) fn note(progress: &dyn Progress, line: &str) {
    progress.write(&format!("  {line}\n"));
}
