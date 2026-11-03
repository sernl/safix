//! Standard-alphabet base64 (RFC 4648 §4), both ways.
//!
//! Three callers, one implementation. [`crate::bitwarden`] encodes a payload
//! for `bw`, padded. [`crate::age_header`] decodes an ASCII-armored age file
//! and an SSH public key's blob, and encodes the four-byte tag age writes into
//! an SSH stanza, unpadded.
//!
//! Written out for the reason [`crate::digest`] is: the locked dependency
//! graph is a decision rather than a lock update, and this is a page of code
//! held to the RFC's own vectors.
//!
//! The decoder is strict. It reads the standard alphabet with or without
//! padding and refuses every other byte — the URL-safe `-` and `_`, whitespace,
//! a line break — and refuses a final group of one character, which carries
//! six bits and so no whole byte, and a final group whose unused bits are not
//! zero. Every input it accepts is therefore the encoding of exactly one byte
//! string, which is what lets a caller compare decoded bytes rather than
//! spellings.

/// The sixty-four characters, in value order.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The base64 of these bytes, padded with `=` to a multiple of four.
#[must_use]
pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut encoded = String::new();
    for chunk in bytes.chunks(3) {
        let first = chunk.first().copied().unwrap_or(0);
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        let triple = (u32::from(first) << 16) | (u32::from(second) << 8) | u32::from(third);
        let sextet = |shift: u32| {
            let at = usize::try_from((triple >> shift) & 0x3F).unwrap_or(0);
            ALPHABET.get(at).copied().map_or('=', char::from)
        };
        encoded.push(sextet(18));
        encoded.push(sextet(12));
        encoded.push(if chunk.len() > 1 { sextet(6) } else { '=' });
        encoded.push(if chunk.len() > 2 { sextet(0) } else { '=' });
    }
    encoded
}

/// The base64 of these bytes with the padding left off, which is the spelling
/// age writes a stanza argument in.
#[must_use]
pub(crate) fn encode_unpadded(bytes: &[u8]) -> String {
    let mut encoded = encode(bytes);
    let kept = encoded.trim_end_matches('=').len();
    encoded.truncate(kept);
    encoded
}

/// The bytes this text encodes, or nothing when it is not standard base64.
///
/// Padding is optional and, when present, must complete the final group to
/// four characters. See the module note for what else is refused.
#[must_use]
pub(crate) fn decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let body = match bytes.len().checked_rem(4)? {
        0 => bytes
            .strip_suffix(b"==")
            .or_else(|| bytes.strip_suffix(b"="))
            .unwrap_or(bytes),
        _ => bytes,
    };
    let mut out = Vec::with_capacity(body.len().saturating_mul(3).checked_div(4)?);
    for group in body.chunks(4) {
        let mut sextets = [0_u32; 4];
        for (slot, &byte) in sextets.iter_mut().zip(group) {
            *slot = value_of(byte)?;
        }
        let [first, second, third, fourth] = sextets;
        let triple = (first << 18) | (second << 12) | (third << 6) | fourth;
        let [_, high, middle, low] = triple.to_be_bytes();
        match group.len() {
            4 => out.extend_from_slice(&[high, middle, low]),
            3 if low == 0 => out.extend_from_slice(&[high, middle]),
            2 if middle == 0 && low == 0 => out.push(high),
            _ => return None,
        }
    }
    Some(out)
}

/// The value of one alphabet character, or nothing for any other byte.
fn value_of(byte: u8) -> Option<u32> {
    ALPHABET
        .iter()
        .position(|&candidate| candidate == byte)
        .and_then(|at| u32::try_from(at).ok())
}

#[cfg(test)]
mod tests {
    use super::{decode, encode, encode_unpadded};

    /// RFC 4648 §10, the independent oracle a hand-written codec needs.
    const VECTORS: [(&[u8], &str); 7] = [
        (b"", ""),
        (b"f", "Zg=="),
        (b"fo", "Zm8="),
        (b"foo", "Zm9v"),
        (b"foob", "Zm9vYg=="),
        (b"fooba", "Zm9vYmE="),
        (b"foobar", "Zm9vYmFy"),
    ];

    #[test]
    fn the_rfc_vectors_encode() {
        for (bytes, text) in VECTORS {
            assert_eq!(encode(bytes), text);
            assert_eq!(encode_unpadded(bytes), text.trim_end_matches('='));
        }
    }

    #[test]
    fn the_rfc_vectors_decode_padded_and_unpadded() {
        for (bytes, text) in VECTORS {
            assert_eq!(decode(text).as_deref(), Some(bytes));
            assert_eq!(decode(text.trim_end_matches('=')).as_deref(), Some(bytes));
        }
    }

    /// The URL-safe alphabet, whitespace, a dangling sextet, padding in the
    /// middle, and a final group carrying bits no byte holds.
    #[test]
    fn anything_outside_the_standard_encoding_is_refused() {
        for text in [
            "Zm9-", "Zm9_", "Zm9 v", "Zm9v\n", " Zm9v", "Zm9vY", "Zg==Zg==", "Zh==", "Zm9=",
            "Zg===", "=",
        ] {
            assert_eq!(decode(text), None, "{text:?} decoded");
        }
    }
}
