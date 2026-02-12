use renskriv_core::scan_all;
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Kunne ikke lese input");

    let spans = scan_all(&input);

    if spans.is_empty() {
        println!("Ingen PII funnet.");
        return;
    }

    println!("Fant {} treff:\n", spans.len());
    for span in &spans {
        println!("  Type:     {:?}", span.pii_type);
        println!("  Verdi:    {}", span.value);
        println!("  Posisjon: {}..{}", span.start, span.end);
        println!();
    }
}
