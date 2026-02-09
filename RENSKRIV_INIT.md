# Renskriv — Project Initialization & Command Reference

## 1. Initialize the Rust Project

```bash
# Create the workspace
cargo new renskriv-core --lib
cd renskriv-core

# Verify it compiles
cargo build
cargo test
```

## 2. Set Up Cargo.toml

Replace the generated `Cargo.toml` with:

```toml
[package]
name = "renskriv-core"
version = "0.1.0"
edition = "2021"
description = "Norwegian PII detection and redaction engine"
license = "MIT"

[lib]
name = "renskriv_core"
crate-type = ["lib"]  # later: "cdylib" for PyO3, add wasm target

[dependencies]
regex = "1"
lazy_static = "1.4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"

[dev-dependencies]
pretty_assertions = "1"
```

## 3. Project Directory Structure

```
renskriv-core/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API, re-exports
│   ├── models.rs           # Detection, Span, PIIType, etc.
│   ├── pipeline.rs         # Orchestrator: text in → detections out
│   ├── patterns/
│   │   ├── mod.rs          # Pattern layer entry point
│   │   ├── fnr.rs          # Fødselsnummer + D-nummer (MOD-11)
│   │   ├── orgnr.rs        # Organisasjonsnummer (MOD-11)
│   │   ├── phone.rs        # Norwegian phone numbers
│   │   ├── email.rs        # Email addresses
│   │   ├── postal.rs       # Postal codes (0001–9999)
│   │   └── bank.rs         # Bank account numbers (MOD-11)
│   ├── ner/
│   │   ├── mod.rs          # NER trait definition
│   │   └── stub.rs         # Placeholder impl (returns empty)
│   ├── gazetteer/
│   │   ├── mod.rs          # Gazetteer lookup trait + impl
│   │   └── data/           # Kartverket data files (later)
│   ├── context/
│   │   └── mod.rs          # Context rules engine (later)
│   └── merger.rs           # Span reconciliation + placeholder IDs
├── tests/
│   ├── test_fnr.rs         # Fødselsnummer tests
│   ├── test_dnr.rs         # D-nummer tests
│   ├── test_orgnr.rs       # Org.nummer tests
│   ├── test_phone.rs       # Phone tests
│   ├── test_email.rs       # Email tests
│   ├── test_postal.rs      # Postal code tests
│   ├── test_bank.rs        # Bank account tests
│   ├── test_merger.rs      # Merger logic tests
│   └── test_pipeline.rs    # Full pipeline integration tests
└── benches/                # Benchmarks (later)
    └── detection_bench.rs
```

Create this structure:

```bash
cd renskriv-core

# Source directories
mkdir -p src/patterns src/ner src/gazetteer/data src/context

# Test directory
mkdir -p tests

# Bench directory
mkdir -p benches

# Create all source files
touch src/models.rs src/pipeline.rs src/merger.rs
touch src/patterns/{mod.rs,fnr.rs,orgnr.rs,phone.rs,email.rs,postal.rs,bank.rs}
touch src/ner/{mod.rs,stub.rs}
touch src/gazetteer/mod.rs
touch src/context/mod.rs

# Create test files
touch tests/{test_fnr.rs,test_dnr.rs,test_orgnr.rs,test_phone.rs,test_email.rs,test_postal.rs,test_bank.rs,test_merger.rs,test_pipeline.rs}
```

---

## 4. Complete Command Reference

### Daily Development

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build

# Test
cargo test                     # Run all tests
cargo test test_fnr            # Run only fødselsnummer tests
cargo test -- --nocapture      # Show println! output in tests
cargo test --lib               # Only unit tests (in src/)
cargo test --test test_fnr     # Only integration test file

# Check (fast — no codegen)
cargo check                    # Type-check without building

# Clippy (linter)
cargo clippy                   # Lint warnings
cargo clippy -- -D warnings    # Treat warnings as errors

# Format
cargo fmt                      # Auto-format all code
cargo fmt -- --check           # Check format without changing

# Docs
cargo doc --open               # Generate and open docs in browser
```

### Testing Workflow (run often)

```bash
# The loop you'll use most:
cargo fmt && cargo clippy -- -D warnings && cargo test

# Or alias it:
alias rcheck='cargo fmt && cargo clippy -- -D warnings && cargo test'
```

### Benchmarks (later)

```bash
cargo bench                    # Run benchmarks
```

### Wasm Target (later, for browser extension)

```bash
# Install wasm target
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack

# Build for wasm
wasm-pack build --target web
```

### PyO3 / Python Bindings (later, for desktop + SpaCy NER)

```bash
# Install maturin
pip install maturin

# Build Python wheel
maturin develop    # Install into current venv
maturin build      # Build wheel for distribution
```

---

## 5. Build Order — What to Implement First

This is your sprint plan. Each step has its own tests. Don't move to the next until tests pass.

### Step 1: Models (`src/models.rs`)
Define the core data types that everything else uses.
- `PIIType` enum (Fodselsnummer, Dnummer, OrgNumber, Phone, Email, PostalCode, BankAccount, Person, Location, Organisation)
- `DetectionSource` enum (Pattern, NER, Gazetteer, Context, Manual)
- `Span` struct (pii_type, value, start, end, confidence, source)
- `RedactionResult` struct
- **No tests needed yet** — these are just data structures

### Step 2: Fødselsnummer Detection (`src/patterns/fnr.rs`)
The single most important detector. Gets you to "something works."
- MOD-11 checksum validation function
- Regex: find all 11-digit sequences in text
- Validate each match with checksum
- Date validation (first 6 digits = valid DDMMYY)
- D-nummer detection (first digit 4–7, subtract 4 for date)
- **Tests:** valid fnr, invalid checksum, D-nummer, 11-digit non-fnr strings, fnr embedded in text

### Step 3: Phone Number Detection (`src/patterns/phone.rs`)
Quick win, simple pattern.
- 8-digit match starting with 4/9 (mobile) or 2/3/5/6/7 (landline)
- Handle +47 prefix, spaces, dashes
- **Tests:** mobile, landline, with/without +47, with spaces, invalid 8-digit sequences

### Step 4: Email Detection (`src/patterns/email.rs`)
Standard but needed.
- RFC 5322 simplified regex
- Norwegian .no domains
- **Tests:** standard addresses, .no domains, edge cases

### Step 5: Postal Code Detection (`src/patterns/postal.rs`)
Simple but needs false-positive prevention.
- 4-digit match, range 0001–9999
- Context awareness: preceded/followed by city name, or standalone
- Avoid matching 4-digit substrings of phone/fnr numbers
- **Tests:** valid codes, codes with city names, false positives from other numbers

### Step 6: Organisasjonsnummer (`src/patterns/orgnr.rs`)
Second MOD-11 pattern.
- 9-digit match starting with 8 or 9
- MOD-11 validation with weights [3, 2, 7, 6, 5, 4, 3, 2]
- **Tests:** valid org numbers, invalid checksums, 9-digit non-org numbers

### Step 7: Bank Account Number (`src/patterns/bank.rs`)
Third MOD-11 pattern.
- 11-digit match (different from fnr — no date prefix)
- MOD-11 on last digit
- **Tests:** valid accounts, invalid checksums, distinguish from fødselsnummer

### Step 8: Pattern Layer Entry Point (`src/patterns/mod.rs`)
Wire all detectors together.
- `detect_all_patterns(text: &str) -> Vec<Span>` runs every detector
- Collects all spans, sorts by position
- **Tests:** multi-pattern text with several PII types

### Step 9: NER Trait (`src/ner/mod.rs` + `src/ner/stub.rs`)
Define the interface, ship a stub.
- `trait NERDetector { fn detect(&self, text: &str) -> Vec<Span>; }`
- `StubNER` returns empty vec (placeholder for SpaCy/Candle later)
- **No real tests yet** — just the interface contract

### Step 10: Merger (`src/merger.rs`)
Combine results from all layers.
- Sort spans by start position
- Resolve overlaps (pattern wins over NER if same span)
- Deduplicate exact matches
- Assign placeholder IDs: `[PERSON_1]`, `[FNUMMER]`, `[PHONE_1]`, etc.
- **Tests:** overlapping spans, duplicates, placeholder assignment, ordering

### Step 11: Pipeline (`src/pipeline.rs`)
Wire everything together.
- `Pipeline` struct holds pattern detectors + NER detector (trait object)
- `fn detect(&self, text: &str) -> RedactionResult`
- Calls patterns → NER → merger → returns result
- **Tests:** full text in → detections out with mock documents

### Step 12: `lib.rs` — Public API
- Re-export: `Pipeline`, `Span`, `PIIType`, `RedactionResult`
- Clean public interface for consumers (Wasm, PyO3, CLI)

---

## 6. Git Setup

```bash
cd renskriv-core
git init
git add .
git commit -m "init: renskriv-core scaffold with pattern detection structure"

# .gitignore
cat > .gitignore << 'EOF'
/target
Cargo.lock
*.swp
*.swo
.DS_Store
EOF

git add .gitignore
git commit -m "chore: add gitignore"
```

---

## 7. Quick Reference Card

| What                  | Command                                              |
|-----------------------|------------------------------------------------------|
| Build                 | `cargo build`                                        |
| Test all              | `cargo test`                                         |
| Test one file         | `cargo test --test test_fnr`                         |
| Test one function     | `cargo test validate_fnr`                            |
| Lint                  | `cargo clippy -- -D warnings`                        |
| Format                | `cargo fmt`                                          |
| Full check cycle      | `cargo fmt && cargo clippy -- -D warnings && cargo test` |
| Docs                  | `cargo doc --open`                                   |
| Release build         | `cargo build --release`                              |
| Add dependency        | `cargo add <crate>`  (needs cargo-edit)              |
| Wasm build (later)    | `wasm-pack build --target web`                       |
| Python wheel (later)  | `maturin develop`                                    |

---

## Next: After Core is Working

Once Steps 1–12 pass all tests with the stub NER:
1. **Gazetteer** — Load Kartverket data, implement address lookup
2. **Context Rules** — Boost confidence near legal keywords
3. **NER real impl** — Either Candle/Tract for Wasm or PyO3+SpaCy for native
4. **CLI wrapper** — `renskriv scan document.txt`
5. **Wasm bindings** — Browser extension target
6. **PyO3 bindings** — Python/Desktop target
