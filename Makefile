src = $(wildcard src/*.rs)

target/debug/autosway: $(src)
	cargo test
	cargo build

install:
	cargo install --path . --force
