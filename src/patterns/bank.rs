use lazy_static::lazy_static;
use regex::Regex;

use crate::model::{DetectionSource, PIIType, Span};
use crate::patterns::fnr;

lazy_static! {
    // 11 digits plain, or formatted as 1234.56.78901
    static ref BANK_RE: Regex = Regex::new(
        r"\b\d{4}\.\d{2}\.\d{5}\b|\b\d{11}\b"
    ).unwrap();
}

// Strip dots and spaces, return exactly 11 digits or None.
fn parse_digits(s: &str) -> Option<[u8; 11]> {
    let digits: Vec<u8> = s
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();

    if digits.len() != 11 {
        return None;
    }

    let mut arr = [0u8; 11];
    arr.copy_from_slice(&digits);
    Some(arr)
}

// Bank account MOD-11: weights [5,4,3,2,7,6,5,4,3,2] on first 10 digits.
// Check digit is digit 11 (index 10). Single check digit, unlike fnr's two.
fn validate_checksum(d: &[u8; 11]) -> bool {
    let weights = [5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
    let sum: u32 = d
        .iter()
        .zip(weights.iter())
        .map(|(&di, &wi)| di as u32 * wi as u32)
        .sum();

    let k = 11 - (sum % 11);
    let k = match k {
        11 => 0,
        10 => return false,
        v => v,
    };

    k == d[10] as u32
}

// Overlap prevention: if the number also passes as a fødselsnummer/D-nummer,
// it belongs to detect_fnr, not to us.
fn is_likely_fnr(d: &[u8; 11]) -> bool {
    fnr::validate_date(d) && fnr::validate_checksum(d)
}

// Intentionally infallible: no matches = empty Vec, not an error.
pub fn detect_bank(text: &str) -> Vec<Span> {
    let mut results = Vec::new();

    for mat in BANK_RE.find_iter(text) {
        let candidate = mat.as_str();

        let digits = match parse_digits(candidate) {
            Some(d) => d,
            None => continue,
        };

        // Bank MOD-11 checksum
        if !validate_checksum(&digits) {
            continue;
        }

        // Overlap: if it's also a valid fnr, let detect_fnr handle it
        if is_likely_fnr(&digits) {
            continue;
        }

        results.push(Span {
            pii_type: PIIType::BankAccount,
            source: DetectionSource::Pattern,
            value: candidate.to_string(),
            start: mat.start(),
            end: mat.end(),
            confidence: 1.0,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bank_checksum_valid() {
        // Verify 12345678903 passes bank MOD-11
        let digits = [1, 2, 3, 4, 5, 6, 7, 8, 9, 0, 3];
        assert!(validate_checksum(&digits));
    }

    #[test]
    fn test_valid_bank_account() {
        let results = detect_bank("Konto: 12345678903");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "12345678903");
        assert!(matches!(results[0].pii_type, PIIType::BankAccount));
    }

    #[test]
    fn test_valid_with_dots() {
        let results = detect_bank("Konto: 1234.56.78903");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_reject_invalid_checksum() {
        let results = detect_bank("Konto: 12345678900");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_fnr_overlap() {
        // 01010101944 is a valid fødselsnummer AND passes bank MOD-11.
        // Should be rejected here — detect_fnr owns it.
        let results = detect_bank("Nummer: 01010101944");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_no_match() {
        let results = detect_bank("Ingen kontonummer her");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_multiple_accounts() {
        let results = detect_bank("A: 12345678903 og B: 86011117947");
        assert_eq!(results.len(), 2);
    }
}
