use crate::model::{DetectionSource, PIIType, Span};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // 9 digits plain, or 3+3+3 with spaces
    static ref ORGNR_RE: Regex = Regex::new(
        r"\b\d{3}\s\d{3}\s\d{3}\b|\b\d{9}\b"
    ).unwrap();
}

// Strip spaces, return exactly 9 digits or None.
fn parse_digits(s: &str) -> Option<[u8; 9]> {
    let digits: Vec<u8> = s
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();

    if digits.len() != 9 {
        return None;
    }

    let mut arr = [0u8; 9];
    arr.copy_from_slice(&digits);
    Some(arr)
}

// MOD-11 checksum: weights [3, 2, 7, 6, 5, 4, 3, 2] on first 8 digits.
// Check digit is digit 9 (index 8).
fn validate_checksum(d: &[u8; 9]) -> bool {
    let weights = [3, 2, 7, 6, 5, 4, 3, 2];
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

    k == d[8] as u32
}

pub fn detect_orgnr(text: &str) -> Vec<Span> {
    let mut results = Vec::new();

    for mat in ORGNR_RE.find_iter(text) {
        let candidate = mat.as_str();

        // Parse — strip spaces, get 9 digits
        let digits = match parse_digits(candidate) {
            Some(d) => d,
            None => continue,
        };

        // First digit must be 8 or 9
        if digits[0] != 8 && digits[0] != 9 {
            continue;
        }

        // MOD-11 checksum
        if !validate_checksum(&digits) {
            continue;
        }

        results.push(Span {
            pii_type: PIIType::OrgNumber,
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
    fn test_valid_orgnr() {
        let results = detect_orgnr("Org: 923456783");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "923456783");
        assert!(matches!(results[0].pii_type, PIIType::OrgNumber));
    }

    #[test]
    fn test_valid_orgnr_with_spaces() {
        let results = detect_orgnr("Org: 923 456 783");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_valid_orgnr_starting_with_8() {
        let results = detect_orgnr("Org: 812345672");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_reject_wrong_first_digit() {
        // Starts with 1 — not a valid orgnr prefix
        let results = detect_orgnr("Nummer: 123456789");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_invalid_checksum() {
        // Valid prefix (9) but wrong check digit
        let results = detect_orgnr("Org: 923456780");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_too_few_digits() {
        let results = detect_orgnr("Kort: 12345678");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_no_match() {
        let results = detect_orgnr("Ingen orgnr her");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_multiple_orgnr() {
        let results = detect_orgnr("A: 923456783 og B: 812345672");
        assert_eq!(results.len(), 2);
    }
}
