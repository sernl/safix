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
/// or a log with no diagnostic at all — which is why that runtime carried four
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

    /// Read one line, without the newline that ended it.
    ///
    /// One byte at a time, deliberately. The two prompts of `set` read two
    /// consecutive lines from one stream, so a reader that buffered ahead would
    /// swallow the confirmation into the first read's buffer and then find the
    /// stream empty — which is why `bash`'s own `read` builtin reads a byte at a
    /// time from a pipe, and why matching it here is the faithful spelling
    /// rather than the slow one.
    ///
    /// `None` when the stream ended before a newline arrived, which is what
    /// `read` returning non-zero means: whatever bytes had arrived are zeroed
    /// rather than returned, because a value the operator did not finish typing
    /// is not a value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SecretRead`] if the stream fails.
    pub fn read_line_from<R: Read>(source: &mut R) -> Result<Option<Self>> {
        let (buffer, filled, complete) =
            read_line_zeroizing(source).map_err(|cause| Error::SecretRead { cause })?;

        if !complete {
            return Ok(None);
        }

        let exact = buffer.get(..filled).ok_or_else(|| Error::SecretRead {
            cause: io::Error::other("read more bytes than the buffer holds"),
        })?;

        Ok(Some(Self(SecretSlice::new(Box::from(exact)))))
    }

    /// Read every complete line up to one holding `marker` alone, keeping the
    /// newline that ended each.
    ///
    /// How a multi-line prompt is answered, and it is the shell runtime's loop:
    /// each line is appended with its newline, the marker line is consumed and
    /// contributes nothing, and a trailing line the stream ended before
    /// terminating is dropped — `read -r` reports failure there and the loop
    /// body never runs, so the partial line never joins the value.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SecretRead`] if the stream fails.
    pub fn read_until_marker<R: Read>(source: &mut R, marker: &str) -> Result<Self> {
        let (buffer, filled) = read_until_marker_zeroizing(source, marker.as_bytes())
            .map_err(|cause| Error::SecretRead { cause })?;

        let exact = buffer.get(..filled).ok_or_else(|| Error::SecretRead {
            cause: io::Error::other("read more bytes than the buffer holds"),
        })?;

        Ok(Self::from_slice(exact))
    }

    /// The members of the JSON object this value holds, each a value of its own.
    ///
    /// [`None`] when the bytes are not a JSON object, which is the refusal a
    /// multi-output generator gets for printing something else. A member that is
    /// a JSON string becomes its unescaped bytes and any other member becomes
    /// its compact JSON text, which is what `jq -j '.[$k]'` prints for each and
    /// therefore what the shell runtime stores.
    ///
    /// # What this does not zero
    ///
    /// The parse goes through `serde_json`, whose intermediate `Value` and its
    /// allocations are not zeroed: a multi-output generator's plaintext is in
    /// this process's heap in a form nothing here reclaims, until the allocator
    /// reuses it. The value that comes back is zeroed on drop; the intermediate
    /// is not, and no claim is made that it is. The shell runtime pipes the same
    /// document through `jq`, which puts it in a second process's heap instead,
    /// so this is narrower rather than clean.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SecretRead`] if the bytes are not valid JSON at all.
    pub fn json_members(&self) -> Result<Option<std::collections::BTreeMap<String, Self>>> {
        let parsed: serde_json::Value =
            serde_json::from_slice(self.0.expose_secret()).map_err(|cause| Error::SecretRead {
                cause: io::Error::other(cause.to_string()),
            })?;

        let serde_json::Value::Object(members) = parsed else {
            return Ok(None);
        };

        let mut split = std::collections::BTreeMap::new();
        for (name, member) in members {
            let rendered = Zeroizing::new(match member {
                serde_json::Value::String(text) => text.into_bytes(),
                other => other.to_string().into_bytes(),
            });
            split.insert(name, Self::from_slice(&rendered));
        }
        Ok(Some(split))
    }

    /// A value from bytes already in hand.
    ///
    /// Private, and it stays private: the type's whole discipline is that a
    /// value enters through a stream, so a constructor taking bytes is one this
    /// module may use to split a document it has just read and nothing outside
    /// it may reach.
    fn from_slice(bytes: &[u8]) -> Self {
        Self(SecretSlice::new(Box::from(bytes)))
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

    /// Write the value as a JSON string, which is the shape `sops set
    /// --value-stdin` takes.
    ///
    /// The second egress, and it exists so that the encoding happens here rather
    /// than in a `jq` subprocess: the shell runtime pipes the value through
    /// `jq -Rs .` on its way to `sops`, which puts a copy of it in a second
    /// process. This produces the same bytes `jq -Rs .` produces, including its
    /// replacement of ill-formed UTF-8 with U+FFFD, and the intermediate is
    /// zeroed when this returns.
    ///
    /// # Errors
    ///
    /// Returns the sink's own failure.
    pub fn write_json_to<W: Write>(&self, sink: &mut W) -> io::Result<()> {
        let text = Zeroizing::new(String::from_utf8_lossy(self.0.expose_secret()).into_owned());
        serde_json::to_writer(sink, text.as_str()).map_err(io::Error::other)
    }

    /// One trailing newline off a single-line value, and nothing off a
    /// multi-line one.
    ///
    /// `openssl rand -base64 32` and every other echo-shaped one-liner ends in a
    /// newline it did not mean, and storing it would put a stray byte in every
    /// consumer; an OpenSSH private key ends in a newline it did mean, and
    /// taking it off produces a file `ssh` refuses to load. The two are
    /// distinguishable — after removing the final newline a single-line value
    /// has none left — so they are distinguished rather than settled one way.
    #[must_use]
    pub fn without_echoed_newline(self) -> Self {
        let bytes = self.0.expose_secret();
        let Some(body) = bytes.strip_suffix(b"\n") else {
            return self;
        };
        if body.contains(&b'\n') {
            return self;
        }
        Self::from_slice(body)
    }

    /// Whether two values are the same, without branching on where they differ.
    ///
    /// The length is compared first and is therefore leaked, which is what
    /// comparing two strings in shell also leaks and is not what this is
    /// defending: the two values being compared are the operator's two entries
    /// of one secret, and the question is whether a mistyped confirmation can be
    /// narrowed down from how long the comparison took.
    #[must_use]
    pub fn equals(&self, other: &Self) -> bool {
        let left = self.0.expose_secret();
        let right = other.0.expose_secret();
        if left.len() != right.len() {
            return false;
        }
        let mut difference = 0_u8;
        for (one, two) in left.iter().zip(right.iter()) {
            difference |= one ^ two;
        }
        difference == 0
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

/// Read up to and including one newline, a byte at a time, into a buffer that is
/// zeroed on every path out.
///
/// Returns the buffer, how much of it is the line without its newline, and
/// whether a newline was reached at all.
fn read_line_zeroizing<R: Read>(source: &mut R) -> io::Result<(Zeroizing<Vec<u8>>, usize, bool)> {
    let mut buffer = Zeroizing::new(vec![0_u8; INITIAL_BUFFER]);
    let mut filled: usize = 0;

    loop {
        if filled == buffer.len() {
            let grown = buffer
                .len()
                .checked_mul(2)
                .ok_or_else(|| io::Error::other("line too long to buffer"))?;

            let mut bigger = Zeroizing::new(vec![0_u8; grown]);
            let head = bigger
                .get_mut(..filled)
                .ok_or_else(|| io::Error::other("grown buffer is shorter than the old one"))?;
            head.copy_from_slice(buffer.as_slice());
            buffer = bigger;
        }

        let window = buffer
            .get_mut(filled..filled.saturating_add(1))
            .ok_or_else(|| io::Error::other("read past the end of the buffer"))?;

        match source.read(window) {
            Ok(0) => return Ok((buffer, filled, false)),
            Ok(_) => {
                if window.first() == Some(&b'\n') {
                    return Ok((buffer, filled, true));
                }
                filled = filled
                    .checked_add(1)
                    .ok_or_else(|| io::Error::other("line too long to buffer"))?;
            }
            Err(cause) if cause.kind() == io::ErrorKind::Interrupted => {}
            Err(cause) => return Err(cause),
        }
    }
}

/// Read whole lines until one holds `marker` alone, into a buffer that is zeroed
/// on every path out.
///
/// Returns the buffer and how much of it is the value. The marker's own line is
/// consumed and contributes nothing, and a final line the stream ended before
/// terminating is left out of the count.
fn read_until_marker_zeroizing<R: Read>(
    source: &mut R,
    marker: &[u8],
) -> io::Result<(Zeroizing<Vec<u8>>, usize)> {
    let mut buffer = Zeroizing::new(vec![0_u8; INITIAL_BUFFER]);
    let mut filled: usize = 0;
    let mut line_start: usize = 0;

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
            buffer = bigger;
        }

        let window = buffer
            .get_mut(filled..filled.saturating_add(1))
            .ok_or_else(|| io::Error::other("read past the end of the buffer"))?;

        match source.read(window) {
            // A line the stream ended before terminating is dropped, which is
            // what `while IFS= read -r line` does with it.
            Ok(0) => return Ok((buffer, line_start)),
            Ok(_) => {
                let ended = window.first() == Some(&b'\n');
                filled = filled
                    .checked_add(1)
                    .ok_or_else(|| io::Error::other("value too large to buffer"))?;
                if !ended {
                    continue;
                }
                let line = buffer
                    .get(line_start..filled.saturating_sub(1))
                    .ok_or_else(|| io::Error::other("the line is outside the buffer"))?;
                if line == marker {
                    return Ok((buffer, line_start));
                }
                line_start = filled;
            }
            Err(cause) if cause.kind() == io::ErrorKind::Interrupted => {}
            Err(cause) => return Err(cause),
        }
    }
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

    fn line(input: &[u8]) -> (Option<Vec<u8>>, Vec<u8>) {
        let mut source = Cursor::new(input.to_vec());
        let read = Secret::read_line_from(&mut source).expect("reading a cursor cannot fail");
        let mut rest = Vec::new();
        source.read_to_end(&mut rest).expect("a cursor can be read");
        let value = read.map(|secret| {
            let mut sink = Vec::new();
            secret
                .write_to(&mut sink)
                .expect("writing a vec cannot fail");
            sink
        });
        (value, rest)
    }

    #[test]
    fn a_line_read_stops_at_the_newline_and_leaves_the_rest_of_the_stream() {
        let (value, rest) = line(b"first\nsecond\n");
        assert_eq!(value.as_deref(), Some(b"first".as_slice()));
        assert_eq!(rest, b"second\n");
    }

    #[test]
    fn a_stream_that_ends_before_a_newline_yields_no_line() {
        assert_eq!(line(b"unfinished").0, None);
        assert_eq!(line(b"").0, None);
    }

    #[test]
    fn an_empty_line_is_a_value_and_not_an_absent_one() {
        assert_eq!(line(b"\nrest").0.as_deref(), Some(b"".as_slice()));
    }

    #[test]
    fn a_line_longer_than_the_buffer_survives_the_growth() {
        let mut input = vec![b'x'; INITIAL_BUFFER.saturating_mul(2).saturating_add(9)];
        let expected = input.clone();
        input.push(b'\n');
        assert_eq!(line(&input).0.as_deref(), Some(expected.as_slice()));
    }

    fn json_of(input: &[u8]) -> String {
        let secret =
            Secret::read_from(&mut Cursor::new(input.to_vec())).expect("a cursor can be read");
        let mut sink = Vec::new();
        secret
            .write_json_to(&mut sink)
            .expect("writing a vec cannot fail");
        String::from_utf8(sink).expect("the encoding is valid utf-8 by construction")
    }

    /// The expected values here are what `printf %s <value> | jq -Rs .` prints,
    /// which is the pipeline the shell runtime hands `sops set --value-stdin`.
    #[test]
    fn the_json_encoding_is_the_one_the_shell_runtime_pipes() {
        assert_eq!(json_of(b"plain"), r#""plain""#);
        assert_eq!(
            json_of(br#"a "quoted" \ back"#),
            r#""a \"quoted\" \\ back""#
        );
        assert_eq!(json_of(b"tab\there"), r#""tab\there""#);
        assert_eq!(json_of(b"bell\x07"), "\"bell\\u0007\"");
        assert_eq!(json_of("caf\u{e9}".as_bytes()), "\"caf\u{e9}\"");
        assert_eq!(json_of(b"\xff"), "\"\u{fffd}\"");
    }

    fn until_eof(input: &[u8]) -> Vec<u8> {
        let secret = Secret::read_until_marker(&mut Cursor::new(input.to_vec()), "EOF")
            .expect("a cursor can be read");
        let mut sink = Vec::new();
        secret
            .write_to(&mut sink)
            .expect("writing a vec cannot fail");
        sink
    }

    #[test]
    fn a_multiline_read_keeps_each_lines_newline_and_drops_the_marker() {
        assert_eq!(until_eof(b"one\ntwo\nEOF\nrest\n"), b"one\ntwo\n");
        assert_eq!(until_eof(b"EOF\n"), b"");
        assert_eq!(until_eof(b""), b"");
    }

    #[test]
    fn a_multiline_read_drops_a_line_the_stream_never_terminated() {
        assert_eq!(until_eof(b"one\nunfinished"), b"one\n");
        assert_eq!(until_eof(b"unfinished"), b"");
    }

    #[test]
    fn a_multiline_read_survives_growing_past_its_buffer() {
        let mut input = vec![b'x'; INITIAL_BUFFER.saturating_mul(2).saturating_add(5)];
        input.push(b'\n');
        let expected = input.clone();
        input.extend_from_slice(b"EOF\n");
        assert_eq!(until_eof(&input), expected);
    }

    #[test]
    fn only_a_single_line_value_loses_its_trailing_newline() {
        let of = |bytes: &[u8]| {
            Secret::read_from(&mut Cursor::new(bytes.to_vec())).expect("a cursor can be read")
        };
        let bytes_of = |secret: Secret| {
            let mut sink = Vec::new();
            secret
                .write_to(&mut sink)
                .expect("writing a vec cannot fail");
            sink
        };
        assert_eq!(
            bytes_of(of(b"one-liner\n").without_echoed_newline()),
            b"one-liner"
        );
        assert_eq!(
            bytes_of(of(b"-----BEGIN-----\nbody\n").without_echoed_newline()),
            b"-----BEGIN-----\nbody\n"
        );
        assert_eq!(
            bytes_of(of(b"no newline").without_echoed_newline()),
            b"no newline"
        );
        assert_eq!(bytes_of(of(b"\n").without_echoed_newline()), b"");
    }

    #[test]
    fn the_members_of_a_json_object_come_back_as_values_of_their_own() {
        let document = Secret::read_from(&mut Cursor::new(
            br#"{"key": "a value\nwith a newline", "count": 3}"#.to_vec(),
        ))
        .expect("a cursor can be read");
        let members = document
            .json_members()
            .expect("the document is valid json")
            .expect("the document is an object");
        assert_eq!(members.keys().collect::<Vec<_>>(), ["count", "key"]);

        let mut sink = Vec::new();
        members
            .get("key")
            .expect("the member is present")
            .write_to(&mut sink)
            .expect("writing a vec cannot fail");
        assert_eq!(sink, b"a value\nwith a newline");

        let mut counted = Vec::new();
        members
            .get("count")
            .expect("the member is present")
            .write_to(&mut counted)
            .expect("writing a vec cannot fail");
        assert_eq!(counted, b"3");
    }

    #[test]
    fn a_document_that_is_not_an_object_is_reported_as_none_and_one_that_is_not_json_fails() {
        let of = |bytes: &[u8]| {
            Secret::read_from(&mut Cursor::new(bytes.to_vec())).expect("a cursor can be read")
        };
        assert!(of(b"[1, 2]").json_members().unwrap().is_none());
        assert!(of(b"\"a string\"").json_members().unwrap().is_none());
        assert!(of(b"not json at all").json_members().is_err());
    }

    #[test]
    fn two_values_are_equal_exactly_when_their_bytes_are() {
        let of = |bytes: &[u8]| {
            Secret::read_from(&mut Cursor::new(bytes.to_vec())).expect("a cursor can be read")
        };
        assert!(of(b"same").equals(&of(b"same")));
        assert!(of(b"").equals(&of(b"")));
        assert!(!of(b"same").equals(&of(b"samf")));
        assert!(!of(b"same").equals(&of(b"same ")));
        assert!(!of(b"").equals(&of(b"a")));
    }
}
