# CLAUDE.md — Renskriv

## What This Project Is

Renskriv is a Norwegian PII (personally identifiable information) detection and redaction tool. It enables safe AI usage by locally stripping sensitive data from documents before they're sent to external services. The primary market is Norwegian law firms and municipal offices needing GDPR-compliant, locally-processed document redaction under Norway's AI Act.

The core thesis: privacy and data sovereignty are the differentiator. All processing runs locally — no data leaves the user's machine.

## Current State

Only the pattern layer is implemented. Names, addresses and dates are **not** detected yet — do not describe the tool as if they were.

- `crates/renskriv-core` — pattern detectors (`patterns/`), data types (`model.rs`), `scan_all` in `lib.rs` (runs all detectors, drops spans contained in a longer one, sorts by position), and the `renskriv-scan` CLI (`src/bin/scan.rs`).
- `crates/renskriv-wasm` — wasm-bindgen wrapper exposing `scan_text` and `version`. Converts byte offsets to char offsets for JavaScript.
- `extension/` — Firefox Manifest V2 extension. `background.js` loads the WASM and handles scan/redact messages; `content.js` intercepts paste and Enter on AI sites and renders the review panel in a Shadow DOM.
- `extension/src/wasm/` is build output (`make wasm`), not checked in.

### Data Models (`model.rs`)

- `PIIType` — Enum: Fodselsnummer, Dnummer, Phone, OrgNumber, BankAccount, Email, PostalCode
- `DetectionSource` — Enum: Pattern, SpacyNER, Gazetteer, ContextRule, Manual (only Pattern is produced today)
- `Span` — Struct: pii_type, source, value, start, end, confidence

## Commands

```bash
cargo test
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
make wasm      # build WASM into extension/src/wasm/ (needs wasm-pack)
make scan      # run the CLI on a sample string
```

## Planned Architecture (not built)

The target is a **four-layer detection pipeline**:

1. **Layer 1 — Pattern Matching** (`patterns/`) — *implemented*. Regex + MOD-11 checksum validation for structured Norwegian identifiers.
2. **Layer 2 — NER** — *planned*. Named entity recognition behind a trait, so each compilation target supplies its own backend (SpaCy `nb_core_news_lg` via PyO3 natively, Candle/Tract for WASM). Detects PER, LOC, ORG, GPE.
3. **Layer 3 — Gazetteer** — *planned*. Lookup against Kartverket address data (street names, municipalities, counties).
4. **Layer 4 — Context Rules** — *planned*. Legal keywords ("klient", "saksøker", "tiltalte") boost confidence, case numbers are excluded, adjacent spans are merged.

A merger would then resolve overlaps across layers (pattern > NER for identical spans, NER kept if broader) and assign consistent placeholder IDs (`[PERSON_1]`, `[FODSELSNUMMER]`), with a pipeline module orchestrating L1→L2→L3→L4→merger.

Planned distribution surfaces beyond the browser extension: desktop app (Tauri + PyO3), web application, fuller CLI.

## Norwegian PII Specifics

These details matter for correct implementation:

- **Fødselsnummer**: 11 digits. First 6 = DDMMYY birthdate. Last 2 are MOD-11 control digits. Individual numbers (digits 7–9) encode gender (odd = male, even = female) and century.
- **D-nummer**: Same format as fødselsnummer but first digit +4 (so day 01 becomes 41). Subtract 4 from first digit before date validation. Same MOD-11 check.
- **Organisasjonsnummer**: 9 digits starting with 8 or 9. MOD-11 weights: [3, 2, 7, 6, 5, 4, 3, 2].
- **Pattern matching alone catches ~60% of PII.** The other 40% (names, addresses in prose, org names) requires NER. Both layers are essential.
- **Accuracy targets** (goals, not measured): >95% precision and >99% recall for patterns, >85% precision and >80% recall for NER. Overall F1 >90%.

## Testing Conventions

- Every pattern detector needs dedicated tests: valid matches, invalid matches, edge cases, false positive prevention.
- **Never use real personal data in tests.** Generate synthetic but realistic Norwegian data with valid checksums and clearly fictional dates.
- Tests live next to the code in `#[cfg(test)]` modules.
- Run `cargo test` and `cargo clippy` before advancing to the next build step.

## Design Principles

- The original document is never modified. Every export creates a new file.
- The user always has full control over what gets redacted before export.
- Security is built in from the start: no network calls after installation, fully offline operation, no data transmitted externally.

## Competitive Context

Renskriv's differentiators vs. established players (Redactable, CaseGuard, Imprima, DeclassifAI): Norwegian-first design, fully local/client-side processing for data sovereignty, and GDPR-conscious positioning for Norwegian public sector orgs that cannot send documents to cloud APIs. The local-processing approach and Norwegian-specific PII types (fødselsnummer, D-nummer, organisasjonsnummer) are the moat.

## Working With Lukas

- Prefers understanding the "why" behind implementation decisions, not just copying syntax. Explain conceptual frameworks and mental models.
- Concept-first, then implementation. Architecture patterns before code.
- Trait-based design is central — explain in terms of interfaces and contracts.
- Currently learning Rust fundamentals (enums, structs, traits). Meet him where he is.
