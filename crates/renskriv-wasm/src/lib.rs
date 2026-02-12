use renskriv_core::scan_all;
use wasm_bindgen::prelude::*;

/// Convert byte offset to char offset (UTF-16 code unit index).
/// Rust regex returns byte positions (UTF-8), but JavaScript
/// uses char indices (UTF-16). For Norwegian characters (ø, å, æ)
/// the byte offset > char offset, which causes corrupt redaction.
fn byte_offset_to_char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].chars().count()
}

/// Scan text for Norwegian personal data.
/// Returns a JavaScript array of span objects:
///   { pii_type, source, value, start, end, confidence }
/// Start/end are char offsets (not byte offsets) so that
/// JavaScript's substring() works correctly.
#[wasm_bindgen]
pub fn scan_text(text: &str) -> JsValue {
    let mut spans = scan_all(text);

    // Convert byte offsets to char offsets for JavaScript
    for span in &mut spans {
        span.start = byte_offset_to_char_offset(text, span.start);
        span.end = byte_offset_to_char_offset(text, span.end);
    }

    serde_wasm_bindgen::to_value(&spans).unwrap_or(JsValue::NULL)
}

/// Return version number (useful for debugging in the extension).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
