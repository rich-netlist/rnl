//! The check of the numbers of the objects without a name (the draft 0.3, 5.9).
//!
//! The record of the solver refers by number, `%#3`, to the objects that no name reaches, and
//! holds a check of how they are made in the field `numbering` of its head. The check has one line
//! for each number, in order:
//!
//! - the tokens of the statement that makes the object, without free text and comments, each as
//!   written, separated by one space ([`crate::syntax::statement_tokens`]);
//! - for an object in the body of a usage, the line of the usage comes first, made in the same way
//!   whether the usage has a number or not, then a tab and the statement of the body as the
//!   definition writes it; for nested usages the outermost comes first;
//! - when the statement makes one object for each member of a set, a tab and the member as a
//!   reference follow: a host instance or net by its name (`#M1`, `&out`), a terminal by its owner
//!   and its number (`#M1:1`), an RNL object by its name or its number (`%x`, `%#3`).
//!
//! The check is the first 16 hexadecimal digits, in lower case, of the SHA-256 of these lines in
//! UTF-8, each ended by a line feed ([`check`]). SHA-256 is implemented here, without a dependency.
//!
//! An interpretation made with [`crate::Options::draft`] gives the numbers of each drawn level in
//! [`crate::Interpretation::numbering`], whatever its version: a writer that appends the record of
//! the solver reads the level again and puts [`level_check`] in its head.

use crate::Interpretation;

/// The numbers of a drawing level as an interpretation made with [`crate::Options::draft`] reads
/// them (5.9).
#[derive(Clone, Debug, PartialEq)]
pub struct LevelNumbering {
    /// The host level, its index in the host document's levels.
    pub level: usize,
    /// One line for each object of the authored records, in the order of their numbers.
    pub authored: Vec<String>,
    /// The check of those lines alone.
    pub check: String,
    /// The records of the solver in the level's own text, in order.
    pub records: Vec<RecordNumbering>,
}

/// The numbers of one record of the solver (5.9): those of the authored records of its level,
/// then those of its own objects.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordNumbering {
    /// Where the record starts.
    pub file: String,
    pub line: u32,
    /// One line for each of its numbers.
    pub lines: Vec<String>,
    /// The check of the lines, which its `numbering` holds when it is up to date.
    pub check: String,
    /// The `numbering` written in its head.
    pub written: Option<String>,
    /// Whether it writes any number.
    pub uses_numbers: bool,
}

/// The check that the record of the solver of a level holds, as the level reads in an
/// interpretation made with [`crate::Options::draft`]: of the lines of its last record of the
/// solver, the authored records' objects and the record's own, or of the authored records' alone
/// when it has none. When a record's `numbering` does not match, its statements that use a number
/// take no effect, but the check is still of the text as written, with every number read. None for
/// a level that is not drawn, or an interpretation made without the draft.
pub fn level_check(i: &Interpretation, level: usize) -> Option<&str> {
    let n = i.numbering.iter().find(|n| n.level == level)?;
    Some(n.records.last().map_or(&n.check, |r| &r.check))
}

/// The check of some lines (5.9): the first 16 hexadecimal digits, in lower case, of the SHA-256
/// of the lines in UTF-8, each ended by a line feed.
pub fn check(lines: &[String]) -> String {
    let mut text = String::new();
    for l in lines {
        text.push_str(l);
        text.push('\n');
    }
    hex(&sha256(text.as_bytes()))[..16].to_string()
}

/// Bytes as hexadecimal digits in lower case.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98,
    0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
    0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8,
    0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819,
    0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
    0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
    0xc67178f2,
];

/// The SHA-256 digest of some bytes (FIPS 180-4).
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] =
        [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    // the message, a 1 bit, zeros up to 56 bytes mod 64, and its length in bits
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_be_bytes());
    for block in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (t, word) in block.chunks_exact(4).enumerate() {
            w[t] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for t in 16..64 {
            let s0 = w[t - 15].rotate_right(7) ^ w[t - 15].rotate_right(18) ^ (w[t - 15] >> 3);
            let s1 = w[t - 2].rotate_right(17) ^ w[t - 2].rotate_right(19) ^ (w[t - 2] >> 10);
            w[t] = w[t - 16].wrapping_add(s0).wrapping_add(w[t - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for t in 0..64 {
            let [a, b, c, d, e, f, g, hh] = v;
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[t]).wrapping_add(w[t]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            v = [t1.wrapping_add(s0.wrapping_add(maj)), a, b, c, d.wrapping_add(t1), e, f, g];
        }
        for (x, y) in h.iter_mut().zip(v) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 32];
    for (k, x) in h.iter().enumerate() {
        out[4 * k..4 * k + 4].copy_from_slice(&x.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_gives_the_standard_vectors() {
        assert_eq!(hex(&sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(
            hex(&sha256(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // across the boundary of a block, and over several
        assert_eq!(hex(&sha256(&[b'a'; 1000])), "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3");
    }

    #[test]
    fn the_check_is_of_the_lines_each_ended_by_a_line_feed() {
        // no lines: the check of the empty text
        assert_eq!(check(&[]), "e3b0c44298fc1c14");
        let lines = vec!["ab".to_string(), "c".to_string()];
        assert_eq!(check(&lines), hex(&sha256(b"ab\nc\n"))[..16]);
        assert_ne!(check(&lines), check(&["abc".to_string()]));
    }
}
