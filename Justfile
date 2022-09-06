build-dotnet:
    cd src/Hyperlight && dotnet build || cd ../../
    cd src/examples/NativeHost && dotnet build || cd ../../../

build-rust:
    cargo build

build-rust-release:
    cargo build --release

build: build-dotnet build-rust
    echo "built all .Net and Rust projects"

test-rust:
    cargo test -- --nocapture

test-dotnet-hl-tests:
    cd src/tests/Hyperlight.Tests && dotnet test || cd ../../../

test-dotnet-nativehost:
    cd src/examples/NativeHost && dotnet test || cd ../../../

test-dotnet: test-dotnet-hl-tests test-dotnet-nativehost

test-capi:
    cd src/hyperlight_host && just run-tests-capi || cd ../../

valgrind-capi:
    cd src/hyperlight_host && just valgrind-tests-capi || cd ../../

test: test-rust test-dotnet valgrind-capi

check:
    cargo check
fmt-check:
    cargo fmt --all -- --check
fmt: 
    cargo fmt
clippy:
    cargo clippy --all-targets --all-features -- -D warnings
