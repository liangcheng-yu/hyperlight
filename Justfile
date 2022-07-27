build:
    cargo build
test:
    cargo test
check:
    cargo check
fmt-check:
    cargo fmt --all -- --check
fmt: 
    cargo fmt
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

