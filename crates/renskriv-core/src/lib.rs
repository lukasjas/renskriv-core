pub mod model;
pub mod patterns;

use model::Span;
use patterns::{bank, email, fnr, orgnr, phone, postal};

/// Run all pattern detectors on the text.
/// Returns matches sorted by position (start offset).
/// Overlapping spans are deduplicated: longer/more precise span wins.
pub fn scan_all(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    spans.extend(fnr::detect_fnr(text));
    spans.extend(phone::detect_phone(text));
    spans.extend(email::detect_email(text));
    spans.extend(postal::detect_postal(text));
    spans.extend(orgnr::detect_orgnr(text));
    spans.extend(bank::detect_bank(text));

    // Sort: longest span first, then highest confidence
    spans.sort_by(|a, b| {
        let len_a = a.end - a.start;
        let len_b = b.end - b.start;
        len_b.cmp(&len_a).then(
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });

    // Remove spans that are entirely within a longer, more precise span.
    // A span is "contained" if [start, end) lies within another span.
    let mut kept: Vec<Span> = Vec::new();
    for span in spans {
        let dominated = kept
            .iter()
            .any(|existing| existing.start <= span.start && span.end <= existing.end);
        if !dominated {
            kept.push(span);
        }
    }

    kept.sort_by_key(|s| s.start);
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use model::PIIType;

    #[test]
    fn test_scan_all_finds_multiple_types() {
        let text = "Ring 01010101944 eller 91234567 e-post ola@firma.no";
        let spans = scan_all(text);
        assert!(
            spans.len() >= 3,
            "Expected at least 3 matches, got {}",
            spans.len()
        );

        // Verify results are sorted by position
        for i in 1..spans.len() {
            assert!(
                spans[i].start >= spans[i - 1].start,
                "Results are not sorted: {} came before {}",
                spans[i].start,
                spans[i - 1].start
            );
        }
    }

    #[test]
    fn test_scan_all_empty_text() {
        let spans = scan_all("");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_scan_all_no_pii() {
        let spans = scan_all("Helt vanlig tekst uten personopplysninger.");
        assert!(spans.is_empty());
    }

    #[test]
    fn test_scan_all_dedup_fnr_over_postal() {
        // "0150" inside a fodselsnummer should not appear as a separate postal code
        let text = "Fnr: 01010101944";
        let spans = scan_all(text);
        let types: Vec<&PIIType> = spans.iter().map(|s| &s.pii_type).collect();
        assert!(types.contains(&&PIIType::Fodselsnummer));
        assert!(
            !types.contains(&&PIIType::PostalCode),
            "Postal code should not appear inside a national identity number"
        );
    }

    #[test]
    fn test_scan_all_dedup_keeps_nonoverlapping() {
        // Postal code that doesn't overlap with fnr should survive
        let text = "Fnr 01010101944 bur i 0150 Oslo";
        let spans = scan_all(text);
        let types: Vec<&PIIType> = spans.iter().map(|s| &s.pii_type).collect();
        assert!(types.contains(&&PIIType::Fodselsnummer));
        assert!(types.contains(&&PIIType::PostalCode));
    }

    #[test]
    fn test_scan_all_covers_all_types() {
        let text = "Fnr 01010101944 tlf +4791234567 org 923456783 konto 12345678903 e-post ola@firma.no postnr 0150";
        let spans = scan_all(text);

        let types: Vec<&PIIType> = spans.iter().map(|s| &s.pii_type).collect();
        assert!(
            types.contains(&&PIIType::Fodselsnummer),
            "Missing Fodselsnummer"
        );
        assert!(types.contains(&&PIIType::Phone), "Missing Phone");
        assert!(types.contains(&&PIIType::OrgNumber), "Missing OrgNumber");
        assert!(
            types.contains(&&PIIType::BankAccount),
            "Missing BankAccount"
        );
        assert!(types.contains(&&PIIType::Email), "Missing Email");
        assert!(types.contains(&&PIIType::PostalCode), "Missing PostalCode");
    }
}
