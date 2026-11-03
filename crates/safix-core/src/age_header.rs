//! What a raw age file's header discloses, read without decrypting it.
//!
//! An age v1 file opens with a header in the clear: an intro line, one stanza
//! per recipient, and a MAC line beginning `---`, after which the payload
//! starts. A stanza is `-> <type> <arguments…>` followed by body lines. What
//! the arguments name depends on the type, and only two types name their
//! recipient at all: an `ssh-ed25519` or `ssh-rsa` stanza's first argument is
//! a four-byte tag of the recipient's SSH public key, which [`ssh_tag`]
//! recomputes from a declared key. An X25519 stanza carries an ephemeral share
//! and nothing about the recipient; a plugin stanza's arguments are the
//! plugin's; an `scrypt` stanza is a passphrase. So the header names each SSH
//! recipient by a tag and every other recipient by nothing.
//!
//! [`stanzas`] reads the header and stops at the MAC line: the payload is never
//! touched, the body lines are skipped rather than decoded, and no identity is
//! consulted. An ASCII-armored file is decoded first and then read exactly as
//! the binary form is. Stanzas whose type ends in `-grease` are dropped — age
//! writes them at random to keep parsers honest, and they name nobody.
//!
//! A tag is thirty-two bits. Anybody able to write the file can mint SSH keys
//! until one's tag collides with a declared key's and substitute that stanza,
//! so a tag that matches says the file is consistent with the declaration and
//! never that it is wrapped to the declared key. [`crate::check`] reports it
//! that way.

use crate::error::{Error, Result};

/// The first line of every age v1 file.
const INTRO: &str = "age-encryption.org/v1";

/// The line an ASCII-armored age file opens with.
const ARMOR_BEGIN: &str = "-----BEGIN AGE ENCRYPTED FILE-----";

/// The line an ASCII-armored age file closes with.
const ARMOR_END: &str = "-----END AGE ENCRYPTED FILE-----";

/// The stanza types whose first argument is an SSH key tag.
pub const SSH_KINDS: [&str; 2] = ["ssh-ed25519", "ssh-rsa"];

/// One recipient stanza of an age header: its type and its first argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stanza {
    /// The stanza type: `X25519`, `ssh-ed25519`, `ssh-rsa`, `scrypt`, a
    /// plugin's name, or anything else a future age writes.
    pub kind: String,
    /// The first argument, which for an SSH stanza is the key's tag.
    pub first_argument: Option<String>,
}

impl Stanza {
    /// Whether this stanza names its recipient by an SSH key tag.
    #[must_use]
    pub fn is_ssh(&self) -> bool {
        SSH_KINDS.contains(&self.kind.as_str())
    }
}

/// Every recipient stanza in this file's header, in header order, grease
/// dropped.
///
/// # Errors
///
/// [`Error::AgeHeaderUnreadable`] when the bytes do not begin with the age v1
/// intro line, when ASCII armor does not decode, or when the header has no MAC
/// line.
pub fn stanzas(bytes: &[u8]) -> Result<Vec<Stanza>> {
    let armored = bytes.trim_ascii_start();
    if armored.starts_with(ARMOR_BEGIN.as_bytes()) {
        let decoded = dearmor(armored)?;
        return binary_stanzas(&decoded);
    }
    binary_stanzas(bytes)
}

/// The age-format tag of a declared SSH recipient, as `(type, tag)`.
///
/// The tag is the first four bytes of SHA-256 over the key's SSH wire form,
/// in unpadded standard base64: six characters. The wire form is exactly the
/// decoded second field of an OpenSSH public key line, so the comment is
/// ignored and nothing about the key is parsed.
///
/// Nothing when the recipient is not an `ssh-ed25519` or `ssh-rsa` line, or
/// when its key field is not base64.
#[must_use]
pub fn ssh_tag(recipient: &str) -> Option<(String, String)> {
    if !crate::ciphertext::is_ssh_recipient(recipient) {
        return None;
    }
    let mut fields = recipient.split_ascii_whitespace();
    let kind = fields.next()?;
    let blob = crate::base64::decode(fields.next()?)?;
    let digest = crate::digest::sha256(&blob);
    let tag = crate::base64::encode_unpadded(digest.first_chunk::<4>()?);
    Some((kind.to_owned(), tag))
}

/// The bytes an ASCII-armored file encodes.
fn dearmor(armored: &[u8]) -> Result<Vec<u8>> {
    let unreadable = Error::AgeHeaderUnreadable {
        reason: "its ASCII armor does not decode",
    };
    let Ok(text) = std::str::from_utf8(armored) else {
        return Err(unreadable);
    };
    let mut lines = text.lines().map(|line| line.trim_end_matches('\r'));
    if lines.next() != Some(ARMOR_BEGIN) {
        return Err(unreadable);
    }
    let mut body = String::new();
    for line in lines {
        if line == ARMOR_END {
            return crate::base64::decode(&body).ok_or(unreadable);
        }
        body.push_str(line);
    }
    Err(unreadable)
}

/// The stanzas of a binary header, read up to the MAC line and no further.
fn binary_stanzas(bytes: &[u8]) -> Result<Vec<Stanza>> {
    let mut lines = bytes.split(|&byte| byte == b'\n');
    if lines.next() != Some(INTRO.as_bytes()) {
        return Err(Error::AgeHeaderUnreadable {
            reason: "it does not begin with the age v1 intro line",
        });
    }
    let mut found = Vec::new();
    for line in lines {
        if line.starts_with(b"---") {
            return Ok(found);
        }
        let Some(stanza) = line.strip_prefix(b"-> ") else {
            continue;
        };
        let Ok(stanza) = std::str::from_utf8(stanza) else {
            return Err(Error::AgeHeaderUnreadable {
                reason: "a stanza line is not text",
            });
        };
        let mut fields = stanza.split(' ');
        let Some(kind) = fields.next().filter(|kind| !kind.is_empty()) else {
            return Err(Error::AgeHeaderUnreadable {
                reason: "a stanza line names no type",
            });
        };
        if kind.ends_with("-grease") {
            continue;
        }
        found.push(Stanza {
            kind: kind.to_owned(),
            first_argument: fields.next().map(str::to_owned),
        });
    }
    Err(Error::AgeHeaderUnreadable {
        reason: "its header has no closing MAC line",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A header age writes for one ed25519 and one RSA recipient, then a MAC
    /// line and payload bytes that are not text.
    fn binary() -> Vec<u8> {
        let mut bytes = b"age-encryption.org/v1\n\
            -> ssh-ed25519 ySQJdg bAjIJIyPF5GzDkegI84A6vNpZAehOxCzhD4hgfBcxW0\n\
            5UsE6Z0Qb7BA3od5KDaCsyfy9Z3P/D+Er8//t5reWLI\n\
            -> ssh-rsa +6Q5OA\n\
            LelJ9r/PYgXZBrIS9sg3dsStEzSwtaJyBDaHWxaOGE4nFm/G1sa4lmQdEeQVL3SQ\n\
            mXi50iazw8qYY51y+TAWSQ\n\
            --- 5x6v1f6TtDu3Vrv6c0n3bCqVQqrOM4Z7ePg+o1fswGI\n"
            .to_vec();
        bytes.extend_from_slice(&[0xff, 0x00, b'\n', b'-', b'>', b' ', 0xfe, b'\n']);
        bytes
    }

    fn expected() -> Vec<Stanza> {
        vec![
            Stanza {
                kind: "ssh-ed25519".into(),
                first_argument: Some("ySQJdg".into()),
            },
            Stanza {
                kind: "ssh-rsa".into(),
                first_argument: Some("+6Q5OA".into()),
            },
        ]
    }

    fn armor(bytes: &[u8]) -> String {
        let encoded = crate::base64::encode(bytes);
        let mut out = format!("\n  {ARMOR_BEGIN}\n");
        for line in encoded.as_bytes().chunks(64) {
            out.push_str(std::str::from_utf8(line).expect("base64 is ASCII"));
            out.push('\n');
        }
        out.push_str(ARMOR_END);
        out.push('\n');
        out
    }

    fn refusal(bytes: &[u8]) -> &'static str {
        match stanzas(bytes) {
            Err(Error::AgeHeaderUnreadable { reason }) => reason,
            other => unreachable!("expected a refusal, got {other:?}"),
        }
    }

    #[test]
    fn parses_a_binary_header() {
        assert_eq!(stanzas(&binary()).expect("the header reads"), expected());
    }

    #[test]
    fn parses_an_armored_header_the_same() {
        let armored = armor(&binary());
        assert_eq!(
            stanzas(armored.as_bytes()).expect("the armor decodes"),
            expected()
        );
    }

    #[test]
    fn drops_grease_stanzas() {
        let bytes = b"age-encryption.org/v1\n\
            -> 4%-grease q(L\n\
            ZmFrZQ\n\
            -> X25519 c2hhcmU\n\
            Ym9keQ\n\
            --- bWFj\n";
        assert_eq!(
            stanzas(bytes).expect("the header reads"),
            vec![Stanza {
                kind: "X25519".into(),
                first_argument: Some("c2hhcmU".into()),
            }]
        );
    }

    /// The payload after the MAC line holds a `-> ` sequence and bytes that are
    /// not text; neither is read.
    #[test]
    fn ignores_the_payload_after_the_mac_line() {
        let stanzas = stanzas(&binary()).expect("the header reads");
        assert_eq!(stanzas.len(), 2);
    }

    #[test]
    fn refuses_a_missing_intro() {
        assert_eq!(
            refusal(b"age-encryption.org/v2\n--- bWFj\n"),
            "it does not begin with the age v1 intro line"
        );
        assert_eq!(refusal(b""), "it does not begin with the age v1 intro line");
    }

    #[test]
    fn refuses_armor_that_does_not_decode() {
        let unterminated = format!("{ARMOR_BEGIN}\nYWdl\n");
        assert_eq!(
            refusal(unterminated.as_bytes()),
            "its ASCII armor does not decode"
        );
        let alphabet = format!("{ARMOR_BEGIN}\nYW-l\n{ARMOR_END}\n");
        assert_eq!(
            refusal(alphabet.as_bytes()),
            "its ASCII armor does not decode"
        );
    }

    #[test]
    fn refuses_a_header_with_no_mac_line() {
        assert_eq!(
            refusal(b"age-encryption.org/v1\n-> X25519 c2hhcmU\nYm9keQ\n"),
            "its header has no closing MAC line"
        );
    }

    /// The expected tags were not computed by this code. Each is the method
    /// agenix uses (`pkgs/agenix.sh:292-300` at 5e5b88a): run
    /// `ssh-keygen -lf` over the public key, take the `SHA256:` field, base64
    /// decode it, keep the first four bytes, and base64 encode them without
    /// padding. For the ed25519 key the field is
    /// `ySQJdg4Mx2QBIhKK8cqMwmwFJ+NCiZqH3K5YbxdDyOM`, giving `ySQJdg`; for the
    /// RSA key it is `+6Q5OJbCDoskbMmcik6W7LMHPf7dYKizNHFZRdk+NJg`, giving
    /// `+6Q5OA`. `age -R` over the same two keys wrote exactly these tags into
    /// the header [`binary`] reproduces.
    #[test]
    fn ssh_tag_matches_ssh_keygen_and_age() {
        assert_eq!(
            ssh_tag(ED25519),
            Some(("ssh-ed25519".into(), "ySQJdg".into()))
        );
        assert_eq!(ssh_tag(RSA), Some(("ssh-rsa".into(), "+6Q5OA".into())));
        let without_comment = ED25519.rsplit_once(' ').map(|(key, _)| key);
        assert_eq!(
            without_comment.and_then(ssh_tag),
            ssh_tag(ED25519),
            "the comment is not part of the key"
        );
    }

    #[test]
    fn ssh_tag_is_nothing_for_anything_else() {
        assert_eq!(
            ssh_tag("age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p"),
            None
        );
        assert_eq!(ssh_tag("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMUA8"), None);
        assert_eq!(ssh_tag("ssh-dss AAAAB3NzaC1kc3M="), None);
    }

    const ED25519: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMUA89S5n5XLYHBkf8mURxfcJhDmBuQ1/4PB7m/AjIGR pinned-ed25519";

    const RSA: &str = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQChYFsOvSeFlVzqrcOWxXx+m6xnX+kU3UCWMLRlgCF/e2RZiDkWeRtAbfB7pWNKh4oHysSsVHphjKoRRiHFhElIRlZH7fJsvZY+nacdfMFCKq903PeW3MScbPmASYZimwAAaSoWF/Gr6Gjot+UD3ZcuGyJ7JQTPCEm4mLeb02LlVw0M1+Oyuf9Z0sfy7BSMsbmaot+ZAczAZrRqa+dzkvKvLMv4pJbXuHKCbG8al2wuDuBIYzPeKsq9ks/WHZlkvdsMa6k+P5nuLISwP17cngu5KDG8Am+Jiv8N2G5WbYCblzmcr//B+FjyQOvT2OfGyjbttNI9ImlVDe7q9sSI7OU/ pinned-rsa";
}
