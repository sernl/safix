//! SHA-256, over bytes that are never a value.
//!
//! The one hash this crate computes, and the whole of what it is for is
//! [`crate::definition`]: a generator's declaration is reduced to one line so
//! that a later edit to it is detectable from the tree alone. No value and no
//! derivative of a value is ever passed here — the callers are the definition
//! record's writer and its reader, and both hand over declarations.
//!
//! # Why it is written out rather than depended on
//!
//! The locked dependency graph is reviewed offline for licence, provenance and
//! duplication, and `rust-supply-chain` states that adding to it is a decision
//! rather than a lock update. A digest of a declaration is not a cryptographic
//! boundary here — the record and the declaration are committed side by side in
//! one repository, so anybody who can edit one can edit the other — but naming
//! something a digest and computing a weak one invites the reader to assume the
//! stronger property. So the standard function is written out, held to the
//! published test vectors below, and costs the graph nothing.
//!
//! # What is deliberate about the shape of the code
//!
//! Every arithmetic operation is spelled `wrapping_*`, because modular addition
//! is what the algorithm specifies rather than a lint being worked around, and
//! the workspace denies the panicking spellings. The message schedule is carried
//! as a sixteen-word window that is destructured by name and rebuilt shifted, so
//! that no step indexes an array: the four words each round reads are
//! `W[t]`, `W[t+1]`, `W[t+9]` and `W[t+14]`, named at the positions FIPS 180-4
//! gives them. The eight working variables that standard calls `a` through `h`
//! are spelled `var_a` through `var_h`, because a name has to be a name here.

use std::fmt::Write as _;

/// The eight initial hash values: the first thirty-two bits of the fractional
/// parts of the square roots of the first eight primes (FIPS 180-4, §5.3.3).
const INITIAL: [u32; 8] = [
    0x6a09_e667,
    0xbb67_ae85,
    0x3c6e_f372,
    0xa54f_f53a,
    0x510e_527f,
    0x9b05_688c,
    0x1f83_d9ab,
    0x5be0_cd19,
];

/// The sixty-four round constants: the first thirty-two bits of the fractional
/// parts of the cube roots of the first sixty-four primes (FIPS 180-4, §4.2.2).
const ROUND: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// How many bytes one block is.
const BLOCK: usize = 64;

/// The SHA-256 of these bytes, as sixty-four lowercase hexadecimal digits.
///
/// Total: there is no input this refuses and no state it can fail in, which is
/// why it returns the digest rather than a result. The length of the message is
/// bounded by the address space, so the bit count cannot overflow the
/// sixty-four-bit field the padding carries it in.
#[must_use]
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut state = INITIAL;
    for block in padded(bytes).chunks_exact(BLOCK) {
        compress(&mut state, block);
    }
    state.iter().fold(String::new(), |mut out, word| {
        // Writing to a `String` cannot fail; the `Result` is `fmt::Write`'s
        // signature rather than a state this can be in.
        let _ = write!(out, "{word:08x}");
        out
    })
}

/// The message, a `0x80` byte, zeroes up to eight short of a block boundary, and
/// the message length in bits as a big-endian sixty-four-bit integer.
fn padded(bytes: &[u8]) -> Vec<u8> {
    // The conversion is total on every platform this compiles for: a message
    // longer than `u64::MAX` bytes cannot be addressed. The fallback is how that
    // is spelled without a panicking construction.
    let bits = u64::try_from(bytes.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    let mut out = Vec::with_capacity(bytes.len().saturating_add(BLOCK).saturating_add(8));
    out.extend_from_slice(bytes);
    out.push(0x80);
    while out.len().wrapping_rem(BLOCK) != BLOCK.saturating_sub(8) {
        out.push(0);
    }
    out.extend_from_slice(&bits.to_be_bytes());
    out
}

/// One block folded into the state.
///
/// `block` is a `chunks_exact(64)` chunk, so it is sixteen four-byte words; a
/// shorter one would leave the window's tail zeroed rather than be rejected,
/// which is unreachable from the one caller and is why nothing here is fallible.
fn compress(state: &mut [u32; 8], block: &[u8]) {
    let mut window = [0_u32; 16];
    for (slot, chunk) in window.iter_mut().zip(block.chunks_exact(4)) {
        *slot = chunk
            .iter()
            .fold(0_u32, |word, &byte| word.wrapping_shl(8) | u32::from(byte));
    }

    let [
        mut var_a,
        mut var_b,
        mut var_c,
        mut var_d,
        mut var_e,
        mut var_f,
        mut var_g,
        mut var_h,
    ] = *state;

    for &constant in &ROUND {
        let [
            w0,
            w1,
            w2,
            w3,
            w4,
            w5,
            w6,
            w7,
            w8,
            w9,
            w10,
            w11,
            w12,
            w13,
            w14,
            w15,
        ] = window;

        let temp1 = var_h
            .wrapping_add(big_sigma1(var_e))
            .wrapping_add(choose(var_e, var_f, var_g))
            .wrapping_add(constant)
            .wrapping_add(w0);
        let temp2 = big_sigma0(var_a).wrapping_add(majority(var_a, var_b, var_c));

        var_h = var_g;
        var_g = var_f;
        var_f = var_e;
        var_e = var_d.wrapping_add(temp1);
        var_d = var_c;
        var_c = var_b;
        var_b = var_a;
        var_a = temp1.wrapping_add(temp2);

        // `W[t+16] = σ1(W[t+14]) + W[t+9] + σ0(W[t+1]) + W[t]`, the schedule's
        // recurrence, with the window advanced one word so the next round reads
        // `W[t+1]` as its own `w0`. The last sixteen rounds compute words no
        // round uses, which costs sixteen additions and keeps the loop one shape.
        let next = small_sigma1(w14)
            .wrapping_add(w9)
            .wrapping_add(small_sigma0(w1))
            .wrapping_add(w0);
        window = [
            w1, w2, w3, w4, w5, w6, w7, w8, w9, w10, w11, w12, w13, w14, w15, next,
        ];
    }

    for (slot, working) in state
        .iter_mut()
        .zip([var_a, var_b, var_c, var_d, var_e, var_f, var_g, var_h])
    {
        *slot = slot.wrapping_add(working);
    }
}

/// `Ch(x, y, z)`, FIPS 180-4 §4.1.2.
const fn choose(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}

/// `Maj(x, y, z)`, FIPS 180-4 §4.1.2.
const fn majority(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}

/// `Σ0(x)`, FIPS 180-4 §4.1.2.
const fn big_sigma0(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}

/// `Σ1(x)`, FIPS 180-4 §4.1.2.
const fn big_sigma1(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}

/// `σ0(x)`, FIPS 180-4 §4.1.2.
const fn small_sigma0(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}

/// `σ1(x)`, FIPS 180-4 §4.1.2.
const fn small_sigma1(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

#[cfg(test)]
mod tests {
    use super::sha256_hex;

    /// The published vectors, which is the independent oracle this needs.
    ///
    /// The expected digests are FIPS 180-4's own examples and the empty-message
    /// value every implementation agrees on; none of them is computed here. The
    /// three lengths are chosen to exercise the padding's branches: a message
    /// that pads inside one block, one that pads to exactly the boundary, and
    /// one that spills into a second block.
    #[test]
    fn the_published_vectors_come_out() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(
            sha256_hex(&vec![b'a'; 1_000_000]),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    /// A message exactly one byte short of needing a second block, and one
    /// exactly at the boundary.
    ///
    /// The padding's `while` loop is where an off-by-one hides, and it hides
    /// specifically at 55, 56 and 64 bytes: at 55 the length field fits, at 56 it
    /// does not and a whole block of padding is appended, and at 64 the message
    /// is a block of its own.
    #[test]
    fn the_padding_boundaries_come_out() {
        assert_eq!(
            sha256_hex(&[b'a'; 55]),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 56]),
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 64]),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
    }

    /// One bit changed changes the digest, which is the property the drift
    /// finding rests on.
    #[test]
    fn a_single_byte_changes_the_digest() {
        assert_ne!(sha256_hex(b"abc"), sha256_hex(b"abd"));
        assert_ne!(sha256_hex(b"abc"), sha256_hex(b"abc "));
        assert_eq!(sha256_hex(b"abc").len(), 64);
    }
}
