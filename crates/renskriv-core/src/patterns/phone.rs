use crate::model::{DetectionSource, PIIType, Span};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // Optional +47/0047 prefix, then 8 digits with optional spaces/dashes
    static ref PHONE_RE: Regex = Regex::new(
        r"(?:\+47[\s\-]?|0047[\s\-]?)\d[\d\s\-]{6,}\d|\b\d[\d\s\-]{6,}\d\b"
    ).unwrap();
}

// Strip country code prefix, then remove spaces/dashes.
// Return the 8 local digits only if exactly 8 remain.
fn strip_formatting(s: &str) -> Option<String> {
    // Remove +47 or 0047 prefix
    let s = s.trim_start_matches('+');
    let s = if s.starts_with("0047") {
        &s[4..]
    } else if s.starts_with("47") && s.len() > 8 {
        &s[2..]
    } else {
        s
    };
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 8 {
        Some(digits)
    } else {
        None
    }
}

// Reject service numbers: first digit must be 2-9
fn is_valid_prefix(first_digit: char) -> bool {
    matches!(first_digit, '2'..='9')
}

pub fn detect_phone(text: &str) -> Vec<Span> {
    let mut results = Vec::new();

    for mat in PHONE_RE.find_iter(text) {
        let candidate = mat.as_str();

        // Strip spaces/dashes, check exactly 8 digits
        let digits = match strip_formatting(candidate) {
            Some(d) => d,
            None => continue,
        };

        // First digit must be 2-9 (reject 0/1 service numbers)
        let first = digits.chars().next().unwrap();
        if !is_valid_prefix(first) {
            continue;
        }

        // Avvis match som er del av ein referansekode (t.d. HR-2019-0041, NTPAY-2025-0412).
        // Ekte telefonnummer har ikkje bokstav eller bindestrek rett for seg.
        let has_country_prefix = candidate.starts_with('+') || candidate.starts_with("0047");
        if !has_country_prefix && mat.start() > 0 {
            let prev = text.as_bytes()[mat.start() - 1];
            if prev == b'-' || prev.is_ascii_alphabetic() {
                continue;
            }
        }

        // Avvis match som har bokstav eller bindestrek rett etter seg.
        // T.d. "2024-1247-A" der "-A" fylgjer.
        if !has_country_prefix && mat.end() < text.len() {
            let next = text.as_bytes()[mat.end()];
            if next == b'-' || next.is_ascii_alphabetic() {
                continue;
            }
        }

        results.push(Span {
            pii_type: PIIType::Phone,
            source: DetectionSource::Pattern,
            value: candidate.to_string(),
            start: mat.start(),
            end: mat.end(),
            confidence: 0.9,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_mobile() {
        let results = detect_phone("Ring 91234567 for info");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "91234567");
    }

    #[test]
    fn test_valid_with_plus47() {
        let results = detect_phone("Tlf: +47 91234567");
        assert_eq!(results.len(), 1);
        assert!(results[0].value.contains("91234567"));
    }

    #[test]
    fn test_valid_with_0047() {
        let results = detect_phone("Ring 004791234567");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_valid_with_spaces() {
        let results = detect_phone("Tlf: 912 34 567");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_landline() {
        let results = detect_phone("Kontor: 22334455");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "22334455");
    }

    #[test]
    fn test_reject_service_number_0() {
        let results = detect_phone("Nummer: 01234567");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_service_number_1() {
        let results = detect_phone("Nummer: 11234567");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_too_few_digits() {
        let results = detect_phone("Kort: 1234567");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_no_match() {
        let results = detect_phone("Ingen telefonnummer her");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_multiple_phones() {
        let results = detect_phone("Ring 91234567 eller 22334455");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_reject_archive_reference() {
        let results = detect_phone("arkivref. HR-2019-0041");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_protocol_reference() {
        let results = detect_phone("protokollnr. NTS-2025-08-13");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_payroll_reference() {
        let results = detect_phone("lønnsnr. NTPAY-2025-0412");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_court_case_reference() {
        let results = detect_phone("HR-2024-1247-A");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_accept_phone_after_text() {
        // Ekte telefonnummer etter vanleg tekst skal framleis matche
        let results = detect_phone("dir.tlf. 90456723");
        assert_eq!(results.len(), 1);
    }
}
