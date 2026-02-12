use renskriv_core::scan_all;
use wasm_bindgen::prelude::*;

/// Konverter byte-offset til char-offset (UTF-16 code unit index).
/// Rust regex returnerer byte-posisjonar (UTF-8), men JavaScript
/// bruker char-indeksar (UTF-16). For norske teikn (ø, å, æ) er
/// byte-offset > char-offset, noko som gir korrupt sladding.
fn byte_offset_to_char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].chars().count()
}

/// Skann tekst for norske personopplysningar.
/// Returnerer ein JavaScript-array med span-objekt:
///   { pii_type, source, value, start, end, confidence }
/// Start/end er char-offsetar (ikkje byte-offsetar) slik at
/// JavaScript sin substring() fungerer korrekt.
#[wasm_bindgen]
pub fn scan_text(text: &str) -> JsValue {
    let mut spans = scan_all(text);

    // Konverter byte-offsetar til char-offsetar for JavaScript
    for span in &mut spans {
        span.start = byte_offset_to_char_offset(text, span.start);
        span.end = byte_offset_to_char_offset(text, span.end);
    }

    serde_wasm_bindgen::to_value(&spans).unwrap_or(JsValue::NULL)
}

/// Returner versjonsnummer (nyttig for feilsoking i utvidelsen).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
