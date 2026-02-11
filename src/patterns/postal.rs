use lazy_static::lazy_static;
use regex::Regex;

use crate::model::{DetectionSource, PIIType, Span};

lazy_static! {
    // 4 digits, optionally preceded by "NO-" (Norwegian postal code prefix)
    static ref POSTAL_RE: Regex = Regex::new(
        r"(?i)\bNO[- ]?\d{4}\b|\b\d{4}\b"
    ).unwrap();
}

// Strip "NO-" or "NO " prefix, return exactly 4 digits or None.
fn parse_digits(s: &str) -> Option<[u8; 4]> {
    let digits: Vec<u8> = s
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c as u8 - b'0')
        .collect();

    if digits.len() != 4 {
        return None;
    }

    let mut arr = [0u8; 4];
    arr.copy_from_slice(&digits);
    Some(arr)
}

// Norwegian postal codes range from 0001 to 9991.
// 0000 is not a valid postal code.
fn is_valid_range(d: &[u8; 4]) -> bool {
    let num = d[0] as u16 * 1000 + d[1] as u16 * 100 + d[2] as u16 * 10 + d[3] as u16;
    num >= 1 && num <= 9991
}

// Intentionally infallible: no matches = empty Vec, not an error.
// Confidence is lower (0.5) because bare 4-digit numbers are ambiguous —
// could be years, quantities, etc. Context rules in a later layer will boost this.
pub fn detect_postal(text: &str) -> Vec<Span> {
    let mut results = Vec::new();

    for mat in POSTAL_RE.find_iter(text) {
        let candidate = mat.as_str();

        let digits = match parse_digits(candidate) {
            Some(d) => d,
            None => continue,
        };

        if !is_valid_range(&digits) {
            continue;
        }

        // Higher confidence if prefixed with NO- (unambiguous)
        let has_prefix = candidate.len() > 4;
        let confidence = if has_prefix { 0.95 } else { 0.5 };

        results.push(Span {
            pii_type: PIIType::PostalCode,
            source: DetectionSource::Pattern,
            value: candidate.to_string(),
            start: mat.start(),
            end: mat.end(),
            confidence,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_oslo() {
        let results = detect_postal("Adresse: 0150 Oslo");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "0150");
        assert!(matches!(results[0].pii_type, PIIType::PostalCode));
    }

    #[test]
    fn test_valid_with_no_prefix() {
        let results = detect_postal("Postnr: NO-0150");
        assert_eq!(results.len(), 1);
        assert!(results[0].confidence > 0.9);
    }

    #[test]
    fn test_valid_tromsoe() {
        let results = detect_postal("Tromsø 9006");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "9006");
    }

    #[test]
    fn test_reject_0000() {
        let results = detect_postal("Kode: 0000");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_no_match() {
        let results = detect_postal("Ingen postnummer her");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_multiple_postal() {
        let results = detect_postal("Fra 0150 Oslo til 5003 Bergen");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_bare_4_digits_low_confidence() {
        let results = detect_postal("Adresse: 0150");
        assert_eq!(results.len(), 1);
        assert!(results[0].confidence < 0.6);
    }
}
