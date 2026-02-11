# AGENTS.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

Renskriv is a Norwegian PII detection and redaction engine. All processing runs locally — no data leaves the user's machine. The target market is Norwegian law firms and municipal offices requiring GDPR-compliant document redaction.

## Build & Test Commands

```bash
cargo build              # Build
cargo test               # Run all tests
cargo test test_fnr      # Run specific test file
cargo test -- --nocapture  # Show test output
cargo clippy             # Lint
cargo fmt                # Format

# Full check cycle
cargo fmt && cargo clippy -- -D warnings && cargo test

# Run CLI scanner
echo "text with PII" | cargo run --bin renskriv-scan
```

## Architecture

Four-layer detection pipeline, results merged into `RedactionResult`:

1. **Pattern Layer** (`src/patterns/`): Regex + MOD-11 checksum validation for structured identifiers
2. **NER Layer** (`src/ner/`): Trait-based interface — SpaCy for native, Candle/Tract for WASM
3. **Gazetteer Layer** (`src/gazetteer/`): Norwegian address lookup via Kartverket data
4. **Context Layer** (`src/context/`): Rule engine boosting confidence near legal keywords

The NER layer is a **trait** so each compilation target provides its own implementation. Pattern/gazetteer/context/merger are pure Rust with no platform dependencies.

### Key Files

- `src/model.rs` — Core types: `PIIType`, `Span`, `DetectionSource`
- `src/patterns/*.rs` — Individual pattern detectors (fnr, phone, email, orgnr, bank, postal)
- `src/bin/scan.rs` — CLI tool wiring all detectors together

## Norwegian PII Specifics

- **Fødselsnummer**: 11 digits, first 6 = DDMMYY birthdate, last 2 are MOD-11 control digits
- **D-nummer**: Same as fødselsnummer but first digit +4 (day 01 → 41). Subtract 4 before date validation
- **Organisasjonsnummer**: 9 digits starting with 8 or 9, MOD-11 weights: `[3, 2, 7, 6, 5, 4, 3, 2]`
- **Bank account**: 11 digits with MOD-11 on last digit (distinct from fødselsnummer — no date prefix)

Pattern matching catches ~60% of PII; NER handles names, addresses in prose, org names.

## Testing Requirements

- Never use real personal data in tests — generate synthetic Norwegian data with valid checksums and fictional dates
- Each pattern detector needs: valid matches, invalid matches, edge cases, false positive prevention
- Run `cargo test` and `cargo clippy` before advancing to next build step

## Design Principles

- Original document never modified — exports create new files
- All UI must be in Norwegian (Bokmål) — labels, buttons, messages, errors
- Fully offline operation after installation, no network calls
