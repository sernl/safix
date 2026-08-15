//! The plaintext value type.

use std::io::{self, Read, Write};

use secrecy::{ExposeSecret as _, SecretSlice};
use zeroize::Zeroizing;

use crate::error::{Error, Result};
use crate::probe::{
    DebugFallback as _, DisplayFallback as _, FromStrFallback as _, FromStringFallback as _,
    Implements, SerializeFallback as _,
};

/// Where the growing read buffer starts. Larger than any value an operator
/// types and than any key an age or ssh identity is spelled as, so the ordinary
/// read never grows and never copies.
const INITIAL_BUFFER: usize = 4096;

/// A plaintext value, in memory, on its way to or from `sops`.
///
/// # What this type refuses to be
///
/// It implements none of `Debug`, `Display` or `serde::Serialize`, so it cannot
/// reach a format string, a panic message, a log line or a JSON document. The
/// absence is not a convention: [the probes below](#compile-time-probes) are
/// `const` assertions, so adding any of the three fails the build.
///
/// The reason is the runtime this replaces. In shell a value is a string, and a
/// string can be spelled into a herestring, a command substitution, an argument
/// or a log with no diagnostic at all — which is why `safix.sh` carries four
/// comments saying do not, one at each place where the natural spelling is the
/// leaking one. Here those four mistakes are absent constructors and absent
/// traits, and the compiler is what enforces them.
///
/// # How a value gets in, and how it gets out
///
/// In through [`Secret::read_from`] or [`Secret::read_from_stdin`], and out
/// through [`Secret::write_to`]. There is no conversion from a `String` or a
/// `&str` — that absence is compiled too — and no accessor returning the bytes,
/// so the only egress is into a writer, in practice the piped standard input of
/// a `sops` child process.
///
/// # What is zeroed, and what cannot be
///
/// The buffer is zeroed when the value is dropped, including when a read fails
/// partway. The read grows its buffer by allocating a new one, copying, and
/// zeroing the old one, rather than by letting `Vec` reallocate — a
/// reallocation copies the bytes into a fresh allocation and frees the old one
/// without zeroing, and `zeroize`'s own documentation for `Vec` says it "cannot
/// ensure that previous reallocations did not leave values on the heap". The
/// final buffer is allocated at exactly the value's length, so the handoff to
/// the secret box does not reallocate either.
///
/// This type does not defend against swap, against a core dump, or against a
/// debugger attached to the process.
///
/// # Compile-time probes
///
/// See the assertions immediately below this type in `secret.rs`.
pub struct Secret(SecretSlice<u8>);

const _: () = assert!(
    !Implements::<Secret>::DEBUG,
    "Secret must not implement Debug"
);
const _: () = assert!(
    !Implements::<Secret>::DISPLAY,
    "Secret must not implement Display"
);
const _: () = assert!(
    !Implements::<Secret>::SERIALIZE,
    "Secret must not implement serde::Serialize"
);
const _: () = assert!(
    !Implements::<Secret>::FROM_STRING,
    "Secret must not be constructible from an owned string"
);
const _: () = assert!(
    !Implements::<Secret>::FROM_STR,
    "Secret must not be constructible from a borrowed string"
);

impl Secret {
    /// Read a value to end of stream.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SecretRead`] if the stream fails. Whatever had been
    /// read is zeroed first, so no partial value survives the failure.
    pub fn read_from<R: Read>(source: &mut R) -> Result<Self> {
        let (buffer, filled) =
            read_zeroizing(source).map_err(|cause| Error::SecretRead { cause })?;

        let exact = buffer.get(..filled).ok_or_else(|| Error::SecretRead {
            cause: io::Error::other("read more bytes than the buffer holds"),
        })?;

        Ok(Self(SecretSlice::new(Box::from(exact))))
    }

    /// Read a value from this process's own standard input, to end of stream.
    ///
    /// # Errors
    ///
    /// As [`Secret::read_from`].
    pub fn read_from_stdin() -> Result<Self> {
        Self::read_from(&mut io::stdin().lock())
    }

    /// Write the value to a sink — in practice the piped standard input of the
    /// backend. This is the type's only egress.
    ///
    /// # Errors
    ///
    /// Returns the sink's own failure.
    pub fn write_to<W: Write>(&self, sink: &mut W) -> io::Result<()> {
        sink.write_all(self.0.expose_secret())
    }

    /// The value's length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.expose_secret().len()
    }

    /// Whether the value is empty, which is what a stream that was already at
    /// its end produces.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Read to end of stream into a buffer that is zeroed on every path out,
/// growing it by hand so that no reallocation leaves a copy behind.
///
/// Returns the buffer and how much of it is the value; the buffer is longer
/// than the value whenever it grew.
fn read_zeroizing<R: Read>(source: &mut R) -> io::Result<(Zeroizing<Vec<u8>>, usize)> {
    let mut buffer = Zeroizing::new(vec![0_u8; INITIAL_BUFFER]);
    let mut filled: usize = 0;

    loop {
        if filled == buffer.len() {
            let grown = buffer
                .len()
                .checked_mul(2)
                .ok_or_else(|| io::Error::other("value too large to buffer"))?;

            let mut bigger = Zeroizing::new(vec![0_u8; grown]);
            let head = bigger
                .get_mut(..filled)
                .ok_or_else(|| io::Error::other("grown buffer is shorter than the old one"))?;
            head.copy_from_slice(buffer.as_slice());

            // The old buffer is dropped here, and dropping it zeroes it. This
            // assignment is the whole reason the read does not use
            // `read_to_end`.
            buffer = bigger;
        }

        let window = buffer
            .get_mut(filled..)
            .ok_or_else(|| io::Error::other("read past the end of the buffer"))?;

        match source.read(window) {
            Ok(0) => break,
            Ok(read) => {
                filled = filled
                    .checked_add(read)
                    .ok_or_else(|| io::Error::other("value too large to buffer"))?;
            }
            Err(cause) if cause.kind() == io::ErrorKind::Interrupted => {}
            Err(cause) => return Err(cause),
        }
    }

    Ok((buffer, filled))
}

#[cfg(test)]
mod tests {
    use std::io::{self, Cursor, Read};

    use super::{INITIAL_BUFFER, Secret};
    use crate::error::Error;

    fn round_trip(input: &[u8]) -> Vec<u8> {
        let secret = Secret::read_from(&mut Cursor::new(input.to_vec()))
            .expect("reading a cursor cannot fail");
        let mut sink = Vec::new();
        secret
            .write_to(&mut sink)
            .expect("writing a vec cannot fail");
        sink
    }

    #[test]
    fn a_value_survives_the_round_trip_unchanged() {
        for input in [
            b"".as_slice(),
            b"a".as_slice(),
            b"a value with\na newline in it".as_slice(),
            b"\x00\xff\x00trailing nul and high bytes\x00".as_slice(),
        ] {
            assert_eq!(round_trip(input), input);
        }
    }

    #[test]
    fn a_value_longer_than_the_buffer_survives_the_growth() {
        let long = vec![b'x'; INITIAL_BUFFER.saturating_mul(3).saturating_add(17)];
        assert_eq!(round_trip(&long), long);
        assert_eq!(
            Secret::read_from(&mut Cursor::new(long.clone()))
                .expect("reading a cursor cannot fail")
                .len(),
            long.len()
        );
    }

    #[test]
    fn an_empty_stream_makes_an_empty_value() {
        let secret =
            Secret::read_from(&mut Cursor::new(Vec::new())).expect("reading a cursor cannot fail");
        assert!(secret.is_empty());
        assert_eq!(secret.len(), 0);
    }

    /// A reader that yields some bytes and then fails, which is the shape a
    /// pipe takes when the writer dies partway.
    struct FailsAfter {
        remaining: usize,
    }

    impl Read for FailsAfter {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.remaining == 0 {
                return Err(io::Error::other("the writer went away"));
            }
            let take = self.remaining.min(buffer.len());
            buffer
                .get_mut(..take)
                .expect("take is bounded by the buffer length")
                .fill(b'z');
            self.remaining = self.remaining.saturating_sub(take);
            Ok(take)
        }
    }

    #[test]
    fn a_read_that_fails_partway_yields_no_value() {
        let outcome = Secret::read_from(&mut FailsAfter { remaining: 128 });
        assert!(matches!(outcome, Err(Error::SecretRead { .. })));
    }

    /// A reader that reports an interruption once, which is not a failure and
    /// must not truncate the value.
    struct InterruptsOnce {
        interrupted: bool,
        payload: Vec<u8>,
    }

    impl Read for InterruptsOnce {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            let take = self.payload.len().min(buffer.len());
            let taken: Vec<u8> = self.payload.drain(..take).collect();
            buffer
                .get_mut(..take)
                .expect("take is bounded by the buffer length")
                .copy_from_slice(&taken);
            Ok(take)
        }
    }

    #[test]
    fn an_interruption_is_retried_rather_than_treated_as_the_end() {
        let secret = Secret::read_from(&mut InterruptsOnce {
            interrupted: false,
            payload: b"not truncated".to_vec(),
        })
        .expect("an interruption is not a failure");

        let mut sink = Vec::new();
        secret
            .write_to(&mut sink)
            .expect("writing a vec cannot fail");
        assert_eq!(sink, b"not truncated");
    }
}
