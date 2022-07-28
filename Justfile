build-net:
    cd src/Hyperlight && dotnet build && cd ../../

build-rust:
    cargo build

build: build-net build-rust
    echo "built all .Net and Rust projects"
    
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

