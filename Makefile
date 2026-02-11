test:
	cargo test

build:
	cargo build

lint:
	cargo clippy

scan:
	echo "Klient 01010101944 ring +4791234567 org 923456783 og vitne 41010101938 møtte i retten 12345678903. ola@firma.no. 0918" | cargo run --bin renskriv-scan
