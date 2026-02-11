# CLAUDE.md — Renskriv

## What This Project Is

Renskriv is a Norwegian PII (personally identifiable information) detection and redaction tool. It enables safe AI usage by locally stripping sensitive data from documents before they're sent to external services. The primary market is Norwegian law firms and municipal offices needing GDPR-compliant, locally-processed document redaction under Norway's AI Act.

The core thesis: privacy and data sovereignty are the differentiator. All processing runs locally — no data leaves the user's machine.

## Architecture Overview

Renskriv uses a **four-layer detection pipeline** implemented in Rust:

1. **Layer 1 — Pattern Matching** (`patterns/`): Deterministic regex + MOD-11 checksum validation for structured Norwegian identifiers (fødselsnummer, D-nummer, organisasjonsnummer, bankkontonummer, phone, email, postal codes).
2. **Layer 2 — NER** (`ner/`): Named entity recognition via a trait-based interface. SpaCy's `nb_core_news_lg` for native targets (via PyO3), Candle/Tract for WASM targets. Detects PER, LOC, ORG, GPE.
3. **Layer 3 — Gazetteer** (`gazetteer/`): Lookup against Norwegian address data from Kartverket (street names, municipalities, counties) via hashmap/trie.
4. **Layer 4 — Context Rules** (`context/`): Rule engine examining surrounding text — legal keywords ("klient", "saksøker", "tiltalte") boost confidence, case numbers are excluded from PII, adjacent spans are merged.

Results from all layers feed into a **merger** (`merger.rs`) that sorts by position, resolves overlaps (pattern > NER for identical spans, NER kept if broader), deduplicates, and assigns consistent placeholder IDs (`[PERSON_1]`, `[FODSELSNUMMER]`, etc.).

The **pipeline** (`pipeline.rs`) orchestrates the full flow: text in → L1→L2→L3→L4→Merger → `RedactionResult` out.

### Key Architectural Principle

The NER layer is a **trait** — the core defines "I need something that takes text and returns spans." Each compilation target provides its own implementation. Regex, gazetteer, context rules, and merger are pure Rust with no platform dependencies — roughly 70–80% of the codebase, written once.

## Core Data Models (`models.rs`)

- `PIIType` — Enum: Fodselsnummer, Dnummer, Phone, Email, PostalCode, OrgNumber, BankAccount, Person, Location, Organisation
- `DetectionSource` — Enum: Pattern, NER, Gazetteer, Context, Manual
- `Span` — Struct: pii_type, value, start, end, confidence, source
- `RedactionResult` — Struct: original, redacted, spans, metadata

## Compilation Targets

One Rust core library, four distribution surfaces:

| Target | Compile | NER Backend | Tech |
|---|---|---|---|
| Browser extension | wasm32 | Candle/Tract | wasm-bindgen |
| Desktop app | native | SpaCy | Tauri + PyO3 |
| Web application | native/wasm | SpaCy/Candle | API / Streamlit |
| CLI | native | SpaCy | `renskriv scan` |

## Tech Stack

- **Language**: Rust (core), Python (NLP integration, prototyping)
- **Rust crates**: `regex`, `lazy_static`, `serde`, `thiserror`
- **Python/NLP**: SpaCy `nb_core_news_lg` for Norwegian NER, integrated via PyO3
- **External data**: Kartverket (Norwegian address validation), MOD-11 algorithms
- **Build**: `cargo` for build/test/clippy, WASM via `wasm-pack`
- **Reference architectures**: redacter-rs, rust-bert, nlprule, Tauri

## Build Order

Implement sequentially. Each step has its own tests — don't proceed until tests pass.

1. `models.rs` — Core data types
2. `fnr.rs` — Fødselsnummer + D-nummer (11 digits, MOD-11, date validation)
3. `phone.rs` — Phone numbers (8 digits, +47, mobile 4/9 prefix)
4. `email.rs` — Email addresses (RFC 5322, .no domains)
5. `postal.rs` — Postal codes (4 digits, 0001–9999, context-aware)
6. `orgnr.rs` — Organisasjonsnummer (9 digits, starts with 8/9, MOD-11)
7. `bank.rs` — Bank account numbers (11 digits, MOD-11 on last digit)
8. `patterns/mod.rs` — Pattern layer complete
9. `ner/mod.rs` — NER trait + stub implementation
10. `merger.rs` — Span merging, dedup, overlap resolution
11. `pipeline.rs` — Full pipeline orchestration
12. `lib.rs` — Public API

**Phases**: Steps 1–8 first (models + all pattern detectors), then 9–12 (NER stub + merger + pipeline + API), then gazetteer → context rules → real NER → CLI → WASM → PyO3.

## Norwegian PII Specifics

These details matter for correct implementation:

- **Fødselsnummer**: 11 digits. First 6 = DDMMYY birthdate. Last 2 are MOD-11 control digits. Individual numbers (digits 7–9) encode gender (odd = male, even = female) and century.
- **D-nummer**: Same format as fødselsnummer but first digit +4 (so day 01 becomes 41). Subtract 4 from first digit before date validation. Same MOD-11 check.
- **Organisasjonsnummer**: 9 digits starting with 8 or 9. MOD-11 weights: [3, 2, 7, 6, 5, 4, 3, 2].
- **Pattern matching alone catches ~60% of PII.** The other 40% (names, addresses in prose, org names) requires NER. Both layers are essential.
- **Accuracy targets**: >95% precision and >99% recall for patterns, >85% precision and >80% recall for NER. Overall F1 >90%.

## Testing Conventions

- Every pattern detector needs dedicated tests: valid matches, invalid matches, edge cases, false positive prevention.
- **Never use real personal data in tests.** Generate synthetic but realistic Norwegian data with valid checksums and clearly fictional dates.
- Integration tests use sample Norwegian documents: legal filings, employment contracts, municipal correspondence.
- Run `cargo test` and `cargo clippy` before advancing to the next build step.

## Design Principles

- The original document is never modified. Every export creates a new file.
- The user always has full control over what gets redacted before export.
- All UI must be in Norwegian (Bokmål). Labels, buttons, messages, tooltips, errors — non-negotiable for the target market.
- Security is built in from the start: no network calls after installation, fully offline operation, no data transmitted externally.

## Competitive Context

Renskriv's differentiators vs. established players (Redactable, CaseGuard, Imprima, DeclassifAI): Norwegian-first design, fully local/client-side processing for data sovereignty, and GDPR-conscious positioning for Norwegian public sector orgs that cannot send documents to cloud APIs. The local-processing approach and Norwegian-specific PII types (fødselsnummer, D-nummer, organisasjonsnummer) are the moat.

## Working With Lukas

- Prefers understanding the "why" behind implementation decisions, not just copying syntax. Explain conceptual frameworks and mental models.
- Concept-first, then implementation. Architecture patterns before code.
- Trait-based design is central — explain in terms of interfaces and contracts.
- Currently learning Rust fundamentals (enums, structs, traits). Meet him where he is.
- Norwegian language in code comments and UI is expected.
