use crate::model::{DetectionSource, PIIType, Span};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref FNR_RE: Regex = Regex::new(r"\b\d{11}\b").unwrap();
}

// Intentionally infallible: no matches = empty Vec, not an error.
// The pipeline layer above will wrap this in Result for IO errors.
pub fn detect_fnr(text: &str) -> Vec<Span> {
    let mut results = Vec::new();

    for mat in FNR_RE.find_iter(text) {
        let candidate = mat.as_str();

        // Parse — fail fast
        let digits = match parse_digits(candidate) {
            Some(d) => d,
            None => continue, // skip, try next candidate
        };

        // Validate date — fail before expensive checksum
        if !validate_date(&digits) {
            continue;
        }

        // Validate checksum — the real proof
        if !validate_checksum(&digits) {
            continue;
        }

        // Classify: D-nummer has day + 40
        let day = digits[0] * 10 + digits[1];
        let pii_type = if day > 40 {
            PIIType::Dnummer
        } else {
            PIIType::Fodselsnummer
        };

        // Emit
        results.push(Span {
            pii_type,
            source: DetectionSource::Pattern,
            value: candidate.to_string(),
            start: mat.start(),
            end: mat.end(),
            confidence: 1.0,
        });
    }

    results
}

// Parse a string of exactly 11 characters into digits.
// Return None if any character isn't a digit.

pub(crate) fn parse_digits(s: &str) -> Option<[u8; 11]> {
    if s.len() != 11 {
        return None;
    }

    let mut a1 = [0u8; 11]; // creates [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

    for (i, c) in s.chars().enumerate() {
        // i is the index (0, 1, 2, ...)
        // c is the character ('0', '1', '0', ...)
        //
        match c.to_digit(10) {
            Some(d) => a1[i] = d as u8, // got a digit, store it
            None => return None,        // not a digit, bail out
        }
    }

    Some(a1)
}

pub(crate) fn validate_checksum(d: &[u8; 11]) -> bool {
    let w1 = [3, 7, 6, 1, 8, 9, 4, 5, 2];
    //"walk through digits and weights side by side,
    // multiply each pair, add up the results."
    let sum1: u32 = d
        .iter()
        .zip(w1.iter()) // pair each digit with its weight
        .map(|(&di, &wi)| di as u32 * wi as u32) //multiply
        .sum();
    //check if digit is 11 - (sum mod 11)
    let k1 = 11 - (sum1 % 11);
    //special cases
    // If k1 = 11 . check digit is 0
    // if k1 = 10, number is invalid (no single digit works)
    let k1 = match k1 {
        11 => 0,
        10 => return false, //invalid - bail out quickly
        other => other,
    };
    if k1 != d[9] as u32 {
        return false;
    }

    //Weights for check digit 2 (K2): applied to digits 0..=9
    let w2 = [5, 4, 3, 2, 7, 6, 5, 4, 3, 2];

    let sum2: u32 = d
        .iter()
        .zip(w2.iter())
        .map(|(&di, &wi)| di as u32 * wi as u32)
        .sum();

    let k2 = 11 - (sum2 % 11);
    let k2 = match k2 {
        11 => 0,
        10 => return false,
        v => v,
    };

    // Check: does our calculated K2 match digit 11 (index 10)?
    k2 == d[10] as u32
}
// How many days does this month have?
// For February we allow 29 for now — leap year check comes later
// when we resolve the full year from the individual number.
fn days_in_month(month: u8) -> u8 {
    match month {
        1 => 31,
        3 => 31,
        5 => 31,
        7 => 31,
        8 => 31,
        10 => 31,
        12 => 31,
        4 => 30,
        6 => 30,
        9 => 30,
        11 => 30,
        2 => 29,
        _ => 0, // invalid month — will cause day check to fail
    }
}

// Validate that the first 6 digits form a real date (DDMMYY).
// For D-nummer the day has 40 added, so we detect and subtract that.
// Returns true if the date is plausible.
pub(crate) fn validate_date(d: &[u8; 11]) -> bool {
    let dd = d[0] * 10 + d[1]; // raw day (may include +40 for D-nummer)
    let mm = d[2] * 10 + d[3]; // month

    // D-nummer: day field has 40 added (so 01 becomes 41)
    let day = if dd > 40 { dd - 40 } else { dd };

    // Month must be 1..=12
    if !(1..=12).contains(&mm) {
        return false;
    }

    // Day must be 1..=max for that month
    if day < 1 || day > days_in_month(mm) {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_checksum() {
        // A known valid test national identity number
        let digits = [0, 1, 0, 1, 0, 1, 0, 1, 9, 4, 4];
        assert!(validate_checksum(&digits));
    }

    #[test]
    fn test_invalid_checksum() {
        // Same number but last digit changed
        let digits = [0, 1, 0, 1, 0, 1, 0, 1, 9, 4, 0];
        assert!(!validate_checksum(&digits));
    }

    // --- detect_fnr tests ---

    #[test]
    fn test_detect_valid_fnr_in_text() {
        let results = detect_fnr("Ring 01010101944 for info");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].value, "01010101944");
        assert_eq!(results[0].start, 5);
        assert_eq!(results[0].end, 16);
        assert!(matches!(results[0].pii_type, PIIType::Fodselsnummer));
    }

    #[test]
    fn test_dnummer_checksum_is_valid() {
        // Prove 41010101938 passes MOD-11 before using it in detection test.
        // Computed via: weights [3,7,6,1,8,9,4,5,2] → k1=3, weights [5,4,3,2,7,6,5,4,3,2] → k2=8.
        let digits = [4, 1, 0, 1, 0, 1, 0, 1, 9, 3, 8];
        assert!(validate_checksum(&digits));
    }

    #[test]
    fn test_detect_dnummer() {
        // 41010101938 — verified valid by test_dnummer_checksum_is_valid above
        let results = detect_fnr("ID: 41010101938");
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].pii_type, PIIType::Dnummer));
    }

    #[test]
    fn test_reject_invalid_checksum() {
        // 11 digits but last digit wrong — checksum fails
        let results = detect_fnr("bad number 01010101940");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_reject_invalid_date() {
        // Month 13 is invalid — 01130100000
        let results = detect_fnr("text 01130100000 here");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_no_match() {
        let results = detect_fnr("no numbers here at all");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_multiple_matches() {
        let text = "First: 01010101944 and second: 01010101944 done";
        let results = detect_fnr(text);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_twelve_digits_no_match() {
        // 12 digits — should NOT match the \b\d{11}\b pattern
        let results = detect_fnr("long number 012345678901");
        assert_eq!(results.len(), 0);
    }
}
