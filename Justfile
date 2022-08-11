build-dotnet:
    cd src/Hyperlight && dotnet build && cd ../../
    cd src/examples/NativeHost && dotnet build && cd ../../../

build-rust:
    cargo build

build: build-dotnet build-rust
    echo "built all .Net and Rust projects"

test-rust:
    cargo test -- --nocapture

test-dotnet:
    cd src/tests/Hyperlight.Tests && dotnet test && cd ../../../

test: test-rust test-dotnet

check:
    cargo check
fmt-check:
    cargo fmt --all -- --check
fmt: 
    cargo fmt
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

