use lazy_static::lazy_static;
use regex::Regex;

use crate::model::{DetectionSource, PIIType, Span};

lazy_static! {
    // RFC 5322 simplified: local-part @ domain
    // Local: letters, digits, ._%+-
    // Domain: letters, digits, .- with a 2+ char TLD
    static ref EMAIL_RE: Regex = Regex::new(
        r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}"
    ).unwrap();
}

// Intentionally infallible: no matches = empty Vec, not an error.
pub fn detect_email(text: &str) -> Vec<Span> {
    let mut results = Vec::new();

    for mat in EMAIL_RE.find_iter(text) {
        results.push(Span {
            pii_type: PIIType::Email,
            source: DetectionSource::Pattern,
            value: mat.as_str().to_string(),
            start: mat.start(),
            end: mat.end(),
            confidence: 0.95,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_email() {
        let results = detect_email("Kontakt ola.nordmann@example.com for info");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "ola.nordmann@example.com");
    }

    #[test]
    fn test_norwegian_domain() {
        let results = detect_email("Send til post@firma.no");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "post@firma.no");
    }

    #[test]
    fn test_plus_addressing() {
        let results = detect_email("Mail: user+tag@gmail.com");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "user+tag@gmail.com");
    }

    #[test]
    fn test_subdomain() {
        let results = detect_email("E-post: klient@avdeling.firma.no");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "klient@avdeling.firma.no");
    }

    #[test]
    fn test_no_match() {
        let results = detect_email("Ingen epost her");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_invalid_no_tld() {
        let results = detect_email("Feil: user@localhost");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_multiple_emails() {
        let results = detect_email("Fra: a@b.no til: c@d.no");
        assert_eq!(results.len(), 2);
    }
}
