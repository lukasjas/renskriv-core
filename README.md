# Renskriv

Local detection and redaction of Norwegian structured identifiers, with a Firefox extension that checks what you paste into AI chat tools before it is sent.

Everything runs on your machine. The detection engine is Rust compiled to WebAssembly; the extension makes no network requests.

> **Status: early prototype.** Renskriv only finds identifiers with a fixed format. It does **not** detect names, street addresses or dates of birth. A text that scans as "clean" can still contain personal data. Do not rely on it as your only safeguard.

## What it detects

| Type | Formats | Validation |
|---|---|---|
| Fødselsnummer | 11 consecutive digits | Date check + both MOD-11 control digits |
| D-nummer | 11 consecutive digits (day + 40) | Date check + both MOD-11 control digits |
| Organisasjonsnummer | `923456783`, `923 456 783` | Starts with 8 or 9, MOD-11 |
| Bank account | `12345678903`, `1234.56.78903`, `1234 56 78903` | MOD-11 |
| Phone | 8 digits, optional `+47` / `0047`, spaces or dashes | First digit 2–9 |
| Email | Standard addresses | Pattern only |
| Postal code | 4 digits, optional `NO-` prefix | Range 0001–9991, some year-context filtering |

## Known limitations

- **No names, addresses or birth dates.** These need NER and a gazetteer, which are planned but not built (see [Roadmap](#roadmap)).
- **Fødselsnummer written with a space** (`010101 01944`) is not detected.
- **Postal codes are noisy.** Any bare 4-digit number can match, so amounts (`4500 kr`) and some years are flagged.
- **Phone false positives.** Other 8-digit numbers, such as case or order numbers, can be flagged as phone numbers.

## The extension

On ChatGPT, Claude, Gemini and Copilot the extension:

1. Intercepts paste and Enter in the chat input.
2. Scans the text locally.
3. If something is found, opens a panel listing each match so you can approve or reject it.
4. Replaces approved matches with placeholders such as `[FODSELSNUMMER]` and passes the redacted text on.

On any other page, clicking the toolbar icon opens the same panel for manual scanning. The extension only has access to that tab after you click.

It is Firefox-only for now (Manifest V2).

### Build and load

Requires [Rust](https://rustup.rs) and [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/).

```bash
rustup target add wasm32-unknown-unknown
make wasm
```

Then in Firefox open `about:debugging#/runtime/this-firefox`, choose **Load Temporary Add-on** and select `extension/manifest.json`.

## The library

```rust
use renskriv_core::scan_all;

let spans = scan_all("Fnr 01010101944, tlf +47 912 34 567");
for span in spans {
    println!("{:?} {} at {}..{}", span.pii_type, span.value, span.start, span.end);
}
```

`start` and `end` are UTF-8 byte offsets. The WASM binding (`renskriv-wasm`) converts them to character offsets for JavaScript.

There is also a small CLI that reads from stdin:

```bash
echo "Kontakt ola@firma.no, konto 1234.56.78903" | cargo run --bin renskriv-scan
```

## Development

```bash
cargo test
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
```

Tests use synthetic data only. Never add real personal data to this repository.

## Layout

```
crates/renskriv-core/   Detection engine (pattern detectors, span model, CLI)
crates/renskriv-wasm/   wasm-bindgen wrapper around the core
extension/              Firefox extension (content script, background, panel UI)
scripts/build-wasm.sh   Builds the WASM bundle into extension/src/wasm/
```

## Roadmap

Only the pattern layer exists today. Planned, in rough order:

1. Detector fixes: spaced fødselsnummer, stricter postal code context.
2. Gazetteer lookup for Norwegian place and street names (Kartverket data).
3. Named entity recognition for people, organisations and locations.
4. Context rules, for example excluding case numbers and boosting confidence near legal keywords.
5. Manifest V3 and Chrome support.

## License

[MIT](LICENSE)
