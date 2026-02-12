use renskriv_core::scan_all;
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .expect("Failed to read input");

    let spans = scan_all(&input);

    if spans.is_empty() {
        println!("No PII found.");
        return;
    }

    println!("Found {} matches:\n", spans.len());
    for span in &spans {
        println!("  Type:     {:?}", span.pii_type);
        println!("  Value:    {}", span.value);
        println!("  Position: {}..{}", span.start, span.end);
        println!();
    }
}
