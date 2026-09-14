//! Ticket ids: ULIDs built from host-provided entropy (TIX-7).

/// Builds a 26-char, uppercase Crockford base32 ULID from a unix-seconds
/// timestamp and 10 bytes of randomness (the `random_bytes` host import).
pub fn new_ulid(now_unix_secs: u64, random: &[u8; 10]) -> String {
    let ms = now_unix_secs.saturating_mul(1000);
    let mut rand_bytes = [0u8; 16];
    rand_bytes[6..16].copy_from_slice(random);
    let rand = u128::from_be_bytes(rand_bytes);
    ::ulid::Ulid::from_parts(ms, rand).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CROCKFORD: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    #[test]
    fn tix_7_new_ulid_is_well_formed() {
        let id = new_ulid(1_700_000_000, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(id.len(), 26);
        assert!(id.chars().all(|c| CROCKFORD.contains(c)));
    }

    #[test]
    fn tix_7_new_ulid_is_deterministic() {
        let a = new_ulid(1_700_000_000, &[9; 10]);
        let b = new_ulid(1_700_000_000, &[9; 10]);
        assert_eq!(a, b);
    }

    #[test]
    fn tix_7_new_ulid_varies_with_randomness() {
        let a = new_ulid(1_700_000_000, &[0; 10]);
        let b = new_ulid(1_700_000_000, &[255; 10]);
        assert_ne!(a, b);
    }
}
