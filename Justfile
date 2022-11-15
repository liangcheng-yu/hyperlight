
alias build-rust-debug := build-rust
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]


init:
    git submodule update --init --recursive

update-dlmalloc:
    curl -Lv -o src/HyperlightGuest/third_party/dlmalloc/malloc.h https://gee.cs.oswego.edu/pub/misc/malloc.h
    curl -Lv -o src/HyperlightGuest/third_party/dlmalloc/malloc.c https://gee.cs.oswego.edu/pub/misc/malloc.c
    cd src/HyperlightGuest/third_party/dlmalloc && git apply --whitespace=nowarn --verbose malloc.patch || cd ../../../..

build-dotnet:
    cd src/Hyperlight && dotnet build || cd ../../
    cd src/examples/NativeHost && dotnet build || cd ../../../

build-rust:
    cargo build

build-rust-release:
    cargo build --release

build: build-rust build-dotnet
    echo "built all .Net and Rust projects"

test-rust:
    cargo test -- --nocapture

test-dotnet-hl:
    cd src/tests/Hyperlight.Tests && dotnet test || cd ../../../

test-dotnet-nativehost:
    cd src/examples/NativeHost && dotnet run -- -nowait || cd ../../../

test-dotnet: test-dotnet-hl test-dotnet-nativehost

test-capi:
    cd src/hyperlight_host && just run-tests-capi || cd ../../

valgrind-capi:
    cd src/hyperlight_host && just valgrind-tests-capi || cd ../../

test: test-rust test-dotnet valgrind-capi test-capi

check:
    cargo check
fmt-check:
    cargo fmt --all -- --check
fmt: 
    cargo fmt
clippy:
    cargo clippy --all-targets --all-features -- -D warnings
