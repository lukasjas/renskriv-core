use lazy_static::lazy_static;
use regex::Regex;

use crate::model::{DetectionSource, PIIType, Span};

lazy_static! {
    // 4 digits, optionally preceded by "NO-" (Norwegian postal code prefix)
    static ref POSTAL_RE: Regex = Regex::new(
        r"(?i)\bNO[- ]?\d{4}\b|\b\d{4}\b"
    ).unwrap();

    // Ord som tyder pa at 4-sifra talet er eit aarstal, ikkje eit postnummer.
    // Sjekkar ordet rett for eller rett etter matchen.
    static ref YEAR_CONTEXT_RE: Regex = Regex::new(
        r"(?i)(?:\b(?:januar|februar|mars|april|mai|juni|juli|august|september|oktober|november|desember|jan|feb|mar|apr|jun|jul|aug|sep|okt|nov|des|i|fra|til|siden|etter|innen|dato|år|year|regnskapsåret|skattemelding|f\.|født)\s*$|\(\s*f\.\s*$)"
    ).unwrap();

    static ref YEAR_CONTEXT_AFTER_RE: Regex = Regex::new(
        r"(?i)^\s*(?:til|og|eller|-|\))"
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

// Sjekk at teiknet ved posisjon ikkje er eit siffer.
// Returnerer true om posisjonen er utanfor teksten (start/slutt).
fn char_at_is_not_digit(text: &str, byte_pos: usize) -> bool {
    if byte_pos >= text.len() {
        return true;
    }
    !text.as_bytes()[byte_pos].is_ascii_digit()
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

        // Avvis om sifra grensar til fleire siffer i teksten.
        // \b hindrar match inne i "01010101944", men ikkje ved
        // "konto 1234.56" der "1234" har ordgrense pa begge sider.
        // Sjekk teiknet rett for og rett etter matchen.
        if mat.start() > 0 && !char_at_is_not_digit(text, mat.start() - 1) {
            continue;
        }
        if !char_at_is_not_digit(text, mat.end()) {
            continue;
        }

        // Avvis om talet er del av ein kode med bindestrek (serienummer, referansar).
        // T.d. "XPS-9520-NRK" eller "HR-2019-0041" — ikkje postnummer.
        if mat.start() > 0 && text.as_bytes()[mat.start() - 1] == b'-' {
            continue;
        }
        if mat.end() < text.len() && text.as_bytes()[mat.end()] == b'-' {
            continue;
        }

        // Sjekk om talet liknar eit aarstal (1900-2099) i aarstal-kontekst.
        // Berre for bare 4-sifra match (ikkje NO-prefiks).
        let has_prefix = candidate.len() > 4;
        if !has_prefix {
            let num = digits[0] as u16 * 1000
                + digits[1] as u16 * 100
                + digits[2] as u16 * 10
                + digits[3] as u16;
            if (1900..=2099).contains(&num) {
                // Sjekk kontekst: ord for eller etter som tyder pa aarstal
                let before = &text[..mat.start()];
                let after = &text[mat.end()..];
                if YEAR_CONTEXT_RE.is_match(before) || YEAR_CONTEXT_AFTER_RE.is_match(after) {
                    continue;
                }
            }
        }

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

    #[test]
    fn test_reject_part_of_longer_number() {
        // 8 siffer utan separator — korkje "5678" eller "1234" skal matche
        let results = detect_postal("ref 56781234");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_start_of_longer_number() {
        // 4 siffer direkte etterfylgt av fleire siffer
        let results = detect_postal("id 12345");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_end_of_longer_number() {
        // Siffer rett for 4-sifra gruppa
        let results = detect_postal("kode 51234");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_accept_postal_after_text() {
        let results = detect_postal("bur i 0150 Oslo sentrum");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "0150");
    }

    #[test]
    fn test_reject_year_with_month_before() {
        let results = detect_postal("januar 2023 til desember 2023");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_year_with_context_before() {
        let results = detect_postal("fra 2024 vedrorende");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_accept_year_range_as_postal_without_context() {
        // 2009 er eit gyldig postnummer (Nordby) — skal matche utan aarskontekst
        let results = detect_postal("adresse 2009 Nordby");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "2009");
    }

    #[test]
    fn test_accept_postal_0155_not_year() {
        // 0155 er utanfor aarstal-intervallet, skal alltid matche
        let results = detect_postal("Storgata 14, 0155 Oslo");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "0155");
    }

    #[test]
    fn test_reject_birth_year_f_dot() {
        let results = detect_postal("Emma Vik (f. 2020) og Oscar Vik (f. 2022)");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_serial_number_with_dashes() {
        let results = detect_postal("serienr. XPS-9520-NRK4872");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_regnskapsaaret() {
        let results = detect_postal("regnskapsåret 2024 en omsetning");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_skattemelding() {
        let results = detect_postal("Skattemelding 2024 for Anders");
        assert_eq!(results.len(), 0);
    }
}
