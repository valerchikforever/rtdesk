//! One-time codes for attended sessions.
//! 9 symbols from a 31-symbol alphabet without look-alikes (0/O, 1/I/L) ≈ 44.6 bits.
//! Only usable as PAKE input, so guessing is online-only and rate limited by the host.

use rand::Rng;

const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
pub const LEN: usize = 9;

pub fn generate() -> String {
    let mut rng = rand::rngs::OsRng;
    (0..LEN).map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char).collect()
}

/// `ABCDEFGHJ` → `ABC-DEF-GHJ`
pub fn display(code: &str) -> String {
    code.as_bytes().chunks(3).map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>().join("-")
}

/// Accepts user input with spaces/dashes/lowercase.
pub fn normalize(input: &str) -> String {
    input.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format() {
        let c = generate();
        assert_eq!(c.len(), LEN);
        assert!(c.bytes().all(|b| ALPHABET.contains(&b)));
        assert_eq!(normalize(&display(&c).to_lowercase()), c);
        assert_ne!(generate(), generate());
    }
}
