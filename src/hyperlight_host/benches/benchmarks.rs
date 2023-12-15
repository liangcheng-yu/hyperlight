use criterion::{criterion_group, criterion_main, Criterion};
use hyperlight_flatbuffers::flatbuffer_wrappers::function_types::{ParameterValue, ReturnType};
use hyperlight_host::sandbox::{MultiUseSandbox, UninitializedSandbox};
use hyperlight_host::sandbox_state::sandbox::EvolvableSandbox;
use hyperlight_host::sandbox_state::transition::Noop;
use hyperlight_host::GuestBinary;
use hyperlight_testing::simple_guest_as_string;

fn guest_call_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("guest_functions");

    let sandbox: MultiUseSandbox = {
        let path = simple_guest_as_string().unwrap();
        let u_sbox =
            UninitializedSandbox::new(GuestBinary::FilePath(path), None, None, None).unwrap();
        u_sbox.evolve(Noop::default())
    }
    .unwrap();

    let mut call_ctx = sandbox.new_call_context();

    group.bench_function("guest_call", |b| {
        b.iter(|| {
            call_ctx
                .call(
                    "Echo",
                    ReturnType::Int,
                    Some(vec![ParameterValue::String("hello\n".to_string())]),
                )
                .unwrap()
        });
    });
    group.finish();
}

fn sandbox_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("sandboxes");

    let create_sandbox = || {
        let sandbox: MultiUseSandbox = {
            let path = simple_guest_path().unwrap();
            let u_sbox =
                UninitializedSandbox::new(GuestBinary::FilePath(path), None, None, None).unwrap();
            u_sbox.evolve(Noop::default()).unwrap()
        };
        sandbox
    };

    group.bench_function("create_sandbox", |b| {
        b.iter_with_large_drop(create_sandbox);
    });

    group.bench_function("create_sandbox_and_drop", |b| {
        b.iter(create_sandbox);
    });

    group.bench_function("create_sandbox_and_call_context", |b| {
        b.iter_with_large_drop(|| create_sandbox().new_call_context());
    });

    group.bench_function("create_sandbox_and_call_context_and_drop", |b| {
        b.iter(|| create_sandbox().new_call_context());
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = guest_call_benchmark, sandbox_benchmark
}
criterion_main!(benches);
