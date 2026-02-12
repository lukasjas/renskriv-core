pub mod model;
pub mod patterns;

use model::Span;
use patterns::{bank, email, fnr, orgnr, phone, postal};

/// Kjor alle monsterdetektor-er pa teksten.
/// Returnerer funn sortert etter posisjon (start-offset).
/// Overlappande spans blir dedupliserte: lengre/meir presis span vinn.
pub fn scan_all(text: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    spans.extend(fnr::detect_fnr(text));
    spans.extend(phone::detect_phone(text));
    spans.extend(email::detect_email(text));
    spans.extend(postal::detect_postal(text));
    spans.extend(orgnr::detect_orgnr(text));
    spans.extend(bank::detect_bank(text));

    // Sorter: lengste span fyrst, deretter hogaste konfidens
    spans.sort_by(|a, b| {
        let len_a = a.end - a.start;
        let len_b = b.end - b.start;
        len_b.cmp(&len_a).then(
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    });

    // Fjern spans som er heilt innanfor ein lengre, meir presis span.
    // Ein span er "contained" om [start, end) ligg innanfor ein anna span.
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
            "Forventet minst 3 funn, fikk {}",
            spans.len()
        );

        // Sjekk at resultater er sortert etter posisjon
        for i in 1..spans.len() {
            assert!(
                spans[i].start >= spans[i - 1].start,
                "Funn er ikke sortert: {} kom for {}",
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
            "Postnr skal ikkje dukke opp inne i eit fodselsnummer"
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
            "Mangler Fodselsnummer"
        );
        assert!(types.contains(&&PIIType::Phone), "Mangler Phone");
        assert!(types.contains(&&PIIType::OrgNumber), "Mangler OrgNumber");
        assert!(
            types.contains(&&PIIType::BankAccount),
            "Mangler BankAccount"
        );
        assert!(types.contains(&&PIIType::Email), "Mangler Email");
        assert!(types.contains(&&PIIType::PostalCode), "Mangler PostalCode");
    }
}
