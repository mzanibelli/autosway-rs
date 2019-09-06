src = $(wildcard src/*.rs)

target/debug/autosway: $(src)
	cargo test
	cargo build
