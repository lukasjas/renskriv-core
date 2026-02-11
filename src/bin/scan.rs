use renskriv_core::patterns::bank::detect_bank;
use renskriv_core::patterns::email::detect_email;
use renskriv_core::patterns::fnr::detect_fnr;
use renskriv_core::patterns::orgnr::detect_orgnr;
use renskriv_core::patterns::phone::detect_phone;
use renskriv_core::patterns::postal::detect_postal;
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Kunne ikke lese input");

    let mut spans = detect_fnr(&input);
    spans.extend(detect_phone(&input));
    spans.extend(detect_orgnr(&input));
    spans.extend(detect_bank(&input));
    spans.extend(detect_email(&input));
    spans.extend(detect_postal(&input));

    if spans.is_empty() {
        println!("Ingen PII funnet.");
        return;
    }

    spans.sort_by_key(|s| s.start);

    println!("Fant {} treff:\n", spans.len());
    for span in &spans {
        println!("  Type:     {:?}", span.pii_type);
        println!("  Verdi:    {}", span.value);
        println!("  Posisjon: {}..{}", span.start, span.end);
        println!();
    }
}
