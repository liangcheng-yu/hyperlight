import 'c.just'
alias build-rust-debug := build-rust

set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]
set dotenv-load := true

set-trace-env-vars := if os() == "windows" { "$env:RUST_LOG='none,hyperlight_host=info';" } else { "RUST_LOG=none,hyperlight_host=info" }
set-env-command := if os() == "windows" { "$env:" } else { "export " }
bin-suffix := if os() == "windows" { ".bat" } else { ".sh" }

default-target := "debug"
simpleguest_source := "src/tests/rust_guests/simpleguest/target/x86_64-pc-windows-msvc"
dummyguest_source := "src/tests/rust_guests/dummyguest/target/x86_64-pc-windows-msvc"
callbackguest_source := "src/tests/rust_guests/callbackguest/target/x86_64-pc-windows-msvc"
rust_guests_bin_dir := "src/tests/rust_guests/bin"

install-vcpkg:
    cd .. && git clone https://github.com/Microsoft/vcpkg.git || cd -
    cd ../vcpkg && ./bootstrap-vcpkg{{ bin-suffix }} && ./vcpkg integrate install || cd -

install-flatbuffers-with-vcpkg: install-vcpkg
    cd ../vcpkg && ./vcpkg install flatbuffers || cd -

update-dlmalloc:
    curl -Lv -o src/HyperlightGuest/third_party/dlmalloc/malloc.h https://gee.cs.oswego.edu/pub/misc/malloc.h
    curl -Lv -o src/HyperlightGuest/third_party/dlmalloc/malloc.c https://gee.cs.oswego.edu/pub/misc/malloc.c
    cd src/HyperlightGuest/third_party/dlmalloc && git apply --whitespace=nowarn --verbose malloc.patch 

# BUILDING
build-rust-guests target=default-target:
    cd src/tests/rust_guests/callbackguest && cargo build --profile={{ if target == "debug" { "dev" } else { target } }} 
    cd src/tests/rust_guests/simpleguest && cargo build --profile={{ if target == "debug" { "dev" } else { target } }} 
    cd src/tests/rust_guests/dummyguest && cargo build --profile={{ if target == "debug" { "dev" } else { target } }} 

move-rust-guests target=default-target:
    cp {{ callbackguest_source }}/{{ target }}/callbackguest.* {{ rust_guests_bin_dir }}/{{ target }}/
    cp {{ simpleguest_source }}/{{ target }}/simpleguest.* {{ rust_guests_bin_dir }}/{{ target }}/
    cp {{ dummyguest_source }}/{{ target }}/dummyguest.* {{ rust_guests_bin_dir }}/{{ target }}/

build-and-move-rust-guests: (build-rust-guests "debug") (move-rust-guests "debug") (build-rust-guests "release") (move-rust-guests "release")

# short aliases rg "rust guests", cg "c guests" for less typing
rg: build-and-move-rust-guests
cg: build-c-guests
guests: rg cg


build-rust target=default-target:
    cargo build --profile={{ if target == "debug" { "dev" } else { target } }}

build: build-rust

# CLEANING
clean: clean-rust

clean-rust: 
    rm {{ if os() == "windows" {"-Force -Exclude .gitkeep"} else {"-f"} }} src/tests/rust_guests/bin/debug/* && rm {{ if os() == "windows" {"-Force -Exclude .gitkeep"} else {"-f"} }} src/tests/rust_guests/bin/release/*
    cargo clean
    cd src/tests/rust_guests/simpleguest && cargo clean
    cd src/tests/rust_guests/dummyguest && cargo clean
    cd src/tests/rust_guests/callbackguest && cargo clean

# TESTING
# Some tests cannot run with other tests, they are marked as ignored so that cargo test works
# there may be tests that we really want to ignore so we cant just use --ignored and we have to
# Specify the test name of the ignored tests that we want to run
test-rust target=default-target features="": (test-rust-int "rust" target features) (test-rust-int "c" target features) (test-seccomp target)
    # unit tests
    cargo test {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }}  --lib
    
    # ignored tests - these tests need to run serially or with specific properties
    cargo test {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }} test_trace -p hyperlight_host --lib  -- --ignored
    cargo test {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }} test_drop  -p hyperlight_host --lib -- --ignored
    cargo test {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }} hypervisor::metrics::tests::test_gather_metrics -p hyperlight_host --lib -- --ignored
    cargo test {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }} sandbox::metrics::tests::test_gather_metrics -p hyperlight_host --lib -- --ignored
    cargo test {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }} test_metrics -p hyperlight_host --lib -- --ignored
    cargo test {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }} --test integration_test log_message -- --ignored
    cargo test {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }} sandbox::uninitialized::tests::test_log_trace -p hyperlight_host --lib -- --ignored
    cargo test {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }} hypervisor::hypervisor_handler::tests::create_1000_sandboxes -p hyperlight_host --lib -- --ignored
    {{ set-trace-env-vars }} cargo test {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }} --lib sandbox::outb::tests::test_log_outb_log -- --ignored

test-seccomp target=default-target:
    # run seccomp test with feature "seccomp" on and off
    cargo test --profile={{ if target == "debug" { "dev" } else { target } }} -p hyperlight_host test_violate_seccomp_filters --lib -- --ignored
    cargo test --profile={{ if target == "debug" { "dev" } else { target } }} -p hyperlight_host test_violate_seccomp_filters --no-default-features --features mshv,kvm --lib -- --ignored

# rust integration tests. guest can either be "rust" or "c"
test-rust-int guest target=default-target features="":
    # integration tests

    # run execute_on_heap test with feature "executable_heap" on and off
    {{if os() == "windows" { "$env:" } else { "" } }}GUEST="{{guest}}"{{if os() == "windows" { ";" } else { "" } }} cargo test --profile={{ if target == "debug" { "dev" } else { target } }} --test integration_test execute_on_heap --features executable_heap -- --ignored
    {{if os() == "windows" { "$env:" } else { "" } }}GUEST="{{guest}}"{{if os() == "windows" { ";" } else { "" } }} cargo test --profile={{ if target == "debug" { "dev" } else { target } }} --test integration_test execute_on_heap -- --ignored
    # run the rest of the integration tests
    {{if os() == "windows" { "$env:" } else { "" } }}GUEST="{{guest}}"{{if os() == "windows" { ";" } else { "" } }} cargo test -p hyperlight_host {{ if features =="" {''} else if features=="no-default-features" {"--no-default-features" } else {"--no-default-features -F " + features } }} --profile={{ if target == "debug" { "dev" } else { target } }} --test '*'

test-rust-feature-compilation-fail target=default-target:
    @# the following should fail on linux because either kvm or msh feature must be specified, which is why the exit code is inverted with an !.
    {{ if os() == "linux" { "! cargo check -p hyperlight_host --no-default-features 2> /dev/null"} else { "" } }}

test target=default-target: (test-rust target)

# RUST LINTING
check:
    cargo check

fmt-check:
    cargo +nightly fmt --all -- --check
    cargo +nightly fmt --manifest-path src/tests/rust_guests/callbackguest/Cargo.toml -- --check
    cargo +nightly fmt --manifest-path src/tests/rust_guests/simpleguest/Cargo.toml -- --check
    cargo +nightly fmt --manifest-path src/tests/rust_guests/dummyguest/Cargo.toml -- --check

fmt-apply:
    cargo +nightly fmt --all
    cargo +nightly fmt --manifest-path src/tests/rust_guests/callbackguest/Cargo.toml
    cargo +nightly fmt --manifest-path src/tests/rust_guests/simpleguest/Cargo.toml
    cargo +nightly fmt --manifest-path src/tests/rust_guests/dummyguest/Cargo.toml

clippy target=default-target:
    cargo clippy --all-targets --all-features --profile={{ if target == "debug" { "dev" } else { target } }} -- -D warnings 

clippy-apply-fix-unix:
    cargo clippy --fix --all 

clippy-apply-fix-windows:
    cargo clippy --target x86_64-pc-windows-msvc --fix --all 

# Verify Minimum Supported Rust Version
verify-msrv:
    ./dev/verify-msrv.sh hyperlight_host hyperlight_guest hyperlight_common

# GEN FLATBUFFERS
gen-all-fbs-rust-code:
    for fbs in `find src -name "*.fbs"`; do flatc -r --rust-module-root-file --gen-all -o ./src/hyperlight_host/src/flatbuffers/ $fbs; done

gen-all-fbs-csharp-code:
    for fbs in `find src -name "*.fbs"`; do flatc -n  --gen-object-api -o ./src/Hyperlight/flatbuffers $fbs; done

gen-all-fbs-c-code:
    for fbs in `find src -name "*.fbs"`; do flatcc -a -o ./src/HyperlightGuest/include/flatbuffers/generated $fbs; done

gen-all-fbs: gen-all-fbs-rust-code gen-all-fbs-c-code gen-all-fbs-csharp-code

# CARGO REGISTRY

# Note: You need to do `az login` before running this command
cargo-login: set-cargo-registry-env
    az account get-access-token --query "join(' ', ['Bearer', accessToken])" --output tsv | cargo login --registry hyperlight_redist
    az account get-access-token --query "join(' ', ['Bearer', accessToken])" --output tsv | cargo login --registry hyperlight_packages

set-cargo-registry-env:
    {{ set-env-command }}CARGO_REGISTRIES_HYPERLIGHT_PACKAGES_INDEX="sparse+https://pkgs.dev.azure.com/AzureContainerUpstream/hyperlight/_packaging/hyperlight_packages_test/Cargo/index/"
    {{ set-env-command }}CARGO_REGISTRIES_HYPERLIGHT_REDIST_INDEX="sparse+https://pkgs.dev.azure.com/AzureContainerUpstream/hyperlight/_packaging/hyperlight_redist/Cargo/index/"

# RUST EXAMPLES
run-rust-examples target=default-target: (build-rust target)
    cargo run --profile={{ if target == "debug" { "dev" } else { target } }} --example metrics
    cargo run --profile={{ if target == "debug" { "dev" } else { target } }} --example metrics --features "function_call_metrics"
    {{ set-trace-env-vars }} cargo run --profile={{ if target == "debug" { "dev" } else { target } }} --example logging

# The two tracing eamples are flaky on windows so we run them on linux only for now, need to figure out why as they run fine locally on windows
run-rust-examples-linux target=default-target: (build-rust target) (run-rust-examples target)
    {{ set-trace-env-vars }} cargo run --profile={{ if target == "debug" { "dev" } else { target } }} --example tracing
    {{ set-trace-env-vars }} cargo run --profile={{ if target == "debug" { "dev" } else { target } }} --example tracing --features "function_call_metrics"

# BENCHMARKING

# Warning: can overwrite previous local benchmarks, so run this before running benchmarks
# Downloads the benchmarks result from the given release tag.
# If tag is not given, defaults to latest release
# Options for os: "Windows", or "Linux"
# Options for Linux hypervisor: "kvm", "hyperv"
# Options for Windows hypervisor: "none"
bench-download os hypervisor tag="":
    gh release download {{ tag }} -D ./target/ -p benchmarks_{{ os }}_{{ hypervisor }}.tar.gz
    mkdir -p target/criterion {{ if os() == "windows" { "-Force" } else { "" } }}
    tar -zxvf target/benchmarks_{{ os }}_{{ hypervisor }}.tar.gz -C target/criterion/ --strip-components=1

# Warning: compares to and then OVERWRITES the given baseline
bench-ci baseline target=default-target:
    cargo bench --profile={{ if target == "debug" { "dev" } else { target } }} -- --verbose --save-baseline {{ baseline }}

bench target=default-target:
    cargo bench --profile={{ if target == "debug" { "dev" } else { target } }} -- --verbose

# FUZZING
fuzz:
    cd src/hyperlight_host && cargo +nightly fuzz run fuzz_target_1