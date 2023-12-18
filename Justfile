
alias build-rust-debug := build-rust
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]
bin-suffix := if os() == "windows" { ".bat" } else { ".sh" }
set-trace-env-vars := if os() == "windows" { "$env:RUST_LOG='none,hyperlight_host=trace';" } else { "RUST_LOG=none,hyperlight_host=trace" }
default-target:= "debug"
# most recent github release that is not "latest". Note that backticks don't work correctly on windows so we use powershell command substitution $() instead
latest-release:= if os() == "windows" {"$(git tag -l --sort=v:refname | select -last 1)"} else {`git tag -l --sort=v:refname | tail -n 1`}

set dotenv-load

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
    cd src/HyperlightGuest/third_party/dlmalloc && git apply --whitespace=nowarn --verbose malloc.patch || cd ../../../..

build-dotnet:
    cd src/Hyperlight && dotnet build || cd ../../
    cd src/examples/NativeHost && dotnet build || cd ../../../

build-rust target=default-target:
    cargo build --verbose --profile={{ if target == "debug" {"dev"} else { target } }}

build: build-rust build-dotnet
    echo "built all .Net and Rust projects"

test-rust target=default-target:
    cargo test --profile={{ if target == "debug" {"dev"} else { target } }} 
    # tracing tests cannot run with other tests they are marked as ignored so that cargo test works
    # there may be tests that we really want to ignore so we cant just use --ignored and we have to specify the test name of the ignored tests that we want to run
    cargo test --profile={{ if target == "debug" {"dev"} else { target } }} test_trace -p hyperlight_host -- --ignored
    cargo test --profile={{ if target == "debug" {"dev"} else { target } }} test_drop  -p hyperlight_host -- --ignored
    cargo test --profile={{ if target == "debug" {"dev"} else { target } }} hypervisor::metrics::tests::test_gather_metrics -p hyperlight_host -- --ignored
    cargo test --profile={{ if target == "debug" {"dev"} else { target } }} sandbox::metrics::tests::test_gather_metrics -p hyperlight_host -- --ignored
    cargo test --profile={{ if target == "debug" {"dev"} else { target } }} test_metrics -p hyperlight_host -- --ignored

test-dotnet-hl target=default-target:
    cd src/tests/Hyperlight.Tests && dotnet test -c {{ target }} || cd ../../../

test-dotnet-nativehost target=default-target:
    cd src/examples/NativeHost && dotnet run -c {{ target }} -- -nowait || cd ../../../

test-dotnet target=default-target: (test-dotnet-hl target) (test-dotnet-nativehost target)

test-capi target=default-target:
    cd src/hyperlight_capi && just run-tests-capi {{ target }} || cd ../../

build-capi target=default-target:
    cd src/hyperlight_capi && just build-tests-capi {{ target }} || cd ../../

valgrind-capi target=default-target:
    cd src/hyperlight_capi && just valgrind-tests-capi {{ target }} || cd ../../

test target=default-target: (test-rust target) (test-dotnet target) (valgrind-capi target) (test-capi target)

check:
    cargo check

fmt-check:
    cargo fmt --all -- --check

fmt: 
    cargo fmt

clippy target=default-target:
    cargo clippy --all-targets --all-features --profile={{ if target == "debug" {"dev"} else { target } }} -- -D warnings

clippy-apply-fix-unix:
    cargo clippy --fix --all

clippy-apply-fix-windows:
    cargo clippy --target x86_64-pc-windows-msvc --fix --all

fmt-apply:
    cargo fmt --all

gen-all-fbs-rust-code:
    for fbs in `find src -name "*.fbs"`; do flatc -r --rust-module-root-file --gen-all -o ./src/hyperlight_host/src/flatbuffers/ $fbs; done
    cargo fmt --all

gen-all-fbs-csharp-code:
    for fbs in `find src -name "*.fbs"`; do flatc -n  --gen-object-api -o ./src/Hyperlight/flatbuffers $fbs; done

gen-all-fbs-c-code:
    for fbs in `find src -name "*.fbs"`; do flatcc -a -o ./src/HyperlightGuest/include/flatbuffers/generated $fbs; done

gen-all-fbs: gen-all-fbs-rust-code gen-all-fbs-c-code gen-all-fbs-csharp-code

cargo-login:
    az account get-access-token --query "join(' ', ['Bearer', accessToken])" --output tsv | cargo login --registry hyperlight_redist
    az account get-access-token --query "join(' ', ['Bearer', accessToken])" --output tsv | cargo login --registry hyperlight_packages

cargo-login-ci:
    echo Basic $(echo -n PAT:$PAT | base64) | cargo login --registry hyperlight_redist
    echo Basic $(echo -n PAT:$PAT | base64) | cargo login --registry hyperlight_packages

cargo-login-ci-windows:
    "Basic " + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes("PAT:" + ($Env:PAT))) | cargo login --registry hyperlight_redist
    "Basic " + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes("PAT:" + ($Env:PAT))) | cargo login --registry hyperlight_packages

run-rust-examples target=default-target: (build-rust target)
    cargo run --profile={{ if target == "debug" {"dev"} else { target } }} --example metrics
    cargo run --profile={{ if target == "debug" {"dev"} else { target } }} --example metrics --features "function_call_metrics"
    {{set-trace-env-vars}} cargo run --profile={{ if target == "debug" {"dev"} else { target } }} --example logging
# the two tracing eamples are flaky on windows so we run them on linux only for now, need to figure out why as they run fine locally on windows
run-rust-examples-linux target=default-target: (build-rust target) (run-rust-examples target)
    {{set-trace-env-vars}} cargo run --profile={{ if target == "debug" {"dev"} else { target } }} --example tracing
    {{set-trace-env-vars}} cargo run --profile={{ if target == "debug" {"dev"} else { target } }} --example tracing --features "function_call_metrics"

# warning, compares to and then OVERWRITES the given baseline
bench-ci baseline target=default-target:
    cargo bench --profile={{ if target == "debug" {"dev"} else { target } }} -- --verbose --save-baseline {{baseline}}

bench target=default-target:
    cargo bench --profile={{ if target == "debug" {"dev"} else { target } }} -- --verbose

# warning, can overwrite previous local benchmarks, so run this before running benchmarks
bench-download os hypervisor tag=latest-release:
    gh release download {{ tag }} -D ./target/ -p benchmarks_{{ os }}_{{ hypervisor }}.tar.gz
    mkdir -p target/criterion {{ if os() == "windows" { "-Force" } else { "" } }}
    tar -zxvf target/benchmarks_{{ os }}_{{ hypervisor }}.tar.gz -C target/criterion/ --strip-components=1
    