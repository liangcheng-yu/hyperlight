use hyperlight_common::flatbuffer_wrappers::function_types::{ParameterValue, ReturnType};
use rand::Rng;
use tracing::{span, Level};
extern crate hyperlight_host;
use hyperlight_host::{
    sandbox::uninitialized::UninitializedSandbox,
    sandbox_state::{sandbox::EvolvableSandbox, transition::Noop},
    GuestBinary, MultiUseSandbox, Result as HyperlightResult,
};
use hyperlight_testing::simple_guest_as_string;
use opentelemetry::{global::shutdown_tracer_provider, KeyValue};
use opentelemetry_otlp::{new_exporter, new_pipeline, WithExportConfig};
use opentelemetry_sdk::{runtime::Tokio, trace, Resource};
use std::error::Error;
use std::io::stdin;
use std::sync::{Arc, Mutex};
use std::thread::{self, spawn, JoinHandle};
use tracing_opentelemetry::layer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

fn fn_writer(_msg: String) -> HyperlightResult<i32> {
    Ok(0)
}

// Shows how to send tracing events to an OTLP collector using the opentelemetry crate.

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let tracer = new_pipeline()
        .tracing()
        .with_exporter(
            new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317/v1/traces"),
        )
        .with_trace_config(
            trace::config().with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "hyperlight_otel_example",
            )])),
        )
        .install_batch(Tokio)?;

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(layer().with_tracer(tracer))
        .try_init()?;
    Ok(run_example()?)
}
fn run_example() -> HyperlightResult<()> {
    // Get the path to a simple guest binary.
    let hyperlight_guest_path =
        simple_guest_as_string().expect("Cannot find the guest binary at the expected location.");

    let mut join_handles: Vec<JoinHandle<HyperlightResult<()>>> = vec![];

    // Construct a new span named "hyperlight otel tracing example" with INFO  level.
    let span = span!(Level::INFO, "hyperlight otel tracing example",);
    let _entered = span.enter();

    let should_exit = Arc::new(Mutex::new(false));

    for i in 0..10 {
        let path = hyperlight_guest_path.clone();
        let exit = Arc::clone(&should_exit);
        let writer_func = Arc::new(Mutex::new(fn_writer));
        let handle = spawn(move || -> HyperlightResult<()> {
            while !*exit.lock().unwrap() {
                // Construct a new span named "hyperlight tracing example thread" with INFO  level.
                let id = Uuid::new_v4();
                let span = span!(
                    Level::INFO,
                    "hyperlight tracing example thread",
                    context = format!("Thread number {} GUID {}", i, id),
                    uuid = %id,
                );
                let _entered = span.enter();

                // Create a new sandbox.
                let usandbox = UninitializedSandbox::new(
                    GuestBinary::FilePath(path.clone()),
                    None,
                    None,
                    Some(&writer_func),
                )?;

                // Initialize the sandbox.

                let no_op = Noop::<UninitializedSandbox, MultiUseSandbox>::default();

                let mut multiuse_sandbox = usandbox.evolve(no_op)?;

                // Call a guest function 5 times to generate some log entries.
                for _ in 0..5 {
                    let result = multiuse_sandbox.call_guest_function_by_name(
                        "Echo",
                        ReturnType::String,
                        Some(vec![ParameterValue::String("a".to_string())]),
                    );
                    assert!(result.is_ok());
                }

                // Define a message to send to the guest.

                let msg = "Hello, World!!\n".to_string();

                // Call a guest function that calls the HostPrint host function 5 times to generate some log entries.
                for _ in 0..5 {
                    let result = multiuse_sandbox.call_guest_function_by_name(
                        "PrintOutput",
                        ReturnType::Int,
                        Some(vec![ParameterValue::String(msg.clone())]),
                    );
                    assert!(result.is_ok());
                }

                // Call a function that gets cancelled by the host function 5 times to generate some log entries.

                for i in 0..5 {
                    let id = Uuid::new_v4();
                    // Construct a new span named "hyperlight tracing call cancellation example thread" with INFO  level.
                    let span = span!(
                        Level::INFO,
                        "hyperlight tracing call cancellation example thread",
                        context = format!("Thread number {} GUID {}", i, id),
                        uuid = %id,
                    );
                    let _entered = span.enter();
                    let mut ctx = multiuse_sandbox.new_call_context();

                    let result = ctx.call("Spin", ReturnType::Void, None);
                    assert!(result.is_err());
                    let result = ctx.finish();
                    assert!(result.is_ok());
                    multiuse_sandbox = result.unwrap();
                }
                let sleep_for = {
                    let mut rng = rand::thread_rng();
                    rng.gen_range(500..3000)
                };
                thread::sleep(std::time::Duration::from_millis(sleep_for));
            }
            Ok(())
        });
        join_handles.push(handle);
    }

    println!("Press enter to exit...");
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    *should_exit.lock().unwrap() = true;
    for join_handle in join_handles {
        let result = join_handle.join();
        assert!(result.is_ok());
    }
    shutdown_tracer_provider();

    Ok(())
}
