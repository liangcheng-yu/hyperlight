import 'c.just'
alias build-rust-debug := build-rust

set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]
set dotenv-load := true

set-trace-env-vars := if os() == "windows" { "$env:RUST_LOG='none,hyperlight_host=info';" } else { "RUST_LOG=none,hyperlight_host=info" }
set-env-command := if os() == "windows" { "$env:" } else { "export " }
bin-suffix := if os() == "windows" { ".bat" } else { ".sh" }

# Note: most recent github release that is not "latest".
# Backticks don't work correctly on windows so we use powershell
# command substitution $() instead

latest-release := if os() == "windows" { "$(git tag -l --sort=v:refname | select -last 2 | select -first 1)" } else { `git tag -l --sort=v:refname | tail -n 2 | head -n 1` }
default-target := "debug"
simpleguest_source := "src/tests/rust_guests/simpleguest/target/x86_64-pc-windows-msvc"
dummyguest_source := "src/tests/rust_guests/dummyguest/target/x86_64-pc-windows-msvc"
callbackguest_source := "src/tests/rust_guests/callbackguest/target/x86_64-pc-windows-msvc"
rust_guests_bin_dir := "src/tests/rust_guests/bin"

# INITIALIZATION/INSTALLATION
init:
    git submodule update --init --recursive

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

build-dotnet:
    cd src/Hyperlight && dotnet build 
    cd src/examples/NativeHost && dotnet build 

build-rust target=default-target:
    cargo build --profile={{ if target == "debug" { "dev" } else { target } }}

build: build-rust build-dotnet
    echo "built all .NET and Rust projects"

# CLEANING
clean: clean-rust

clean-rust: 
    rm {{ if os() == "windows" {"-Force -Exclude .gitkeep"} else {"-f"} }} src/tests/rust_guests/bin/debug/* && rm {{ if os() == "windows" {"-Force -Exclude .gitkeep"} else {"-f"} }} src/tests/rust_guests/bin/release/*
    cargo clean
    cd src/tests/rust_guests/simpleguest && cargo clean
    cd src/tests/rust_guests/dummyguest && cargo clean
    cd src/tests/rust_guests/callbackguest && cargo clean

# TESTING
# Tracing tests cannot run with other tests they are marked as ignored so that cargo test works
# there may be tests that we really want to ignore so we cant just use --ignored and we have to

# Specify the test name of the ignored tests that we want to run
test-rust target=default-target:
    cargo test --profile={{ if target == "debug" { "dev" } else { target } }} 
    cargo test --profile={{ if target == "debug" { "dev" } else { target } }} test_trace -p hyperlight_host -- --ignored 
    cargo test --profile={{ if target == "debug" { "dev" } else { target } }} test_drop  -p hyperlight_host -- --ignored 
    cargo test --profile={{ if target == "debug" { "dev" } else { target } }} hypervisor::metrics::tests::test_gather_metrics -p hyperlight_host -- --ignored 
    cargo test --profile={{ if target == "debug" { "dev" } else { target } }} sandbox::metrics::tests::test_gather_metrics -p hyperlight_host -- --ignored 
    cargo test --profile={{ if target == "debug" { "dev" } else { target } }} test_metrics -p hyperlight_host -- --ignored 
    cargo test --profile={{ if target == "debug" { "dev" } else { target } }} --test integration_test log_message -- --ignored

test-dotnet-hl target=default-target:
    cd src/tests/Hyperlight.Tests && dotnet test -c {{ target }}

test-dotnet-nativehost target=default-target:
    cd src/examples/NativeHost && dotnet run -c {{ target }} -- -nowait 

test-dotnet-nativehost-c-guests target=default-target:
    cd src/examples/NativeHost && dotnet run -c {{ target }} -- -nowait -usecguests

test-dotnet-hl-c-guests target=default-target:
    [Environment]::SetEnvironmentVariable('guesttype','c', 'Process') && cd src/tests/Hyperlight.Tests && dotnet test -c {{ target }}

test-dotnet-c-guests target=default-target: (test-dotnet-hl-c-guests target) (test-dotnet-nativehost-c-guests target)

test-dotnet target=default-target: (build-hyperlight-surrogate target) (test-dotnet-hl target) (test-dotnet-nativehost target)

build-hyperlight-surrogate target=default-target:
    msbuild -m hyperlight.sln /p:Configuration={{ target }} /t:HyperlightSurrogate

test-capi target=default-target:
    cd src/hyperlight_capi && just run-tests-capi {{ target }} 

build-capi target=default-target:
    cd src/hyperlight_capi && just build-tests-capi {{ target }} 

valgrind-capi target=default-target:
    cd src/hyperlight_capi && just valgrind-tests-capi {{ target }} 

test target=default-target: (test-rust target) (test-dotnet target) (valgrind-capi target) (test-capi target)

# RUST LINTING
check:
    cargo check

fmt-check:
    cargo fmt --all -- --check

fmt-apply:
    cargo fmt --all    

clippy target=default-target:
    cargo clippy --all-targets --all-features --profile={{ if target == "debug" { "dev" } else { target } }} -- -D warnings 

clippy-apply-fix-unix:
    cargo clippy --fix --all 

clippy-apply-fix-windows:
    cargo clippy --target x86_64-pc-windows-msvc --fix --all 

# GEN FLATBUFFERS
gen-all-fbs-rust-code:
    for fbs in `find src -name "*.fbs"`; do flatc -r --rust-module-root-file --gen-all -o ./src/hyperlight_host/src/flatbuffers/ $fbs; done
    cargo fmt --all

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

cargo-login-ci: set-cargo-registry-env
    echo Basic $(echo -n PAT:$PAT | base64) | cargo login --registry hyperlight_redist
    echo Basic $(echo -n PAT:$PAT | base64) | cargo login --registry hyperlight_packages

cargo-login-ci-windows: set-cargo-registry-env
    "Basic " + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes("PAT:" + ($Env:PAT))) | cargo login --registry hyperlight_redist
    "Basic " + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes("PAT:" + ($Env:PAT))) | cargo login --registry hyperlight_packages

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
bench-download os hypervisor tag=latest-release:
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