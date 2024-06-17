use hyperlight_host::MultiUseSandbox;
use hyperlight_host::{
    func::HostFunction1,
    sandbox_state::{sandbox::EvolvableSandbox, transition::Noop},
    GuestBinary, Result, UninitializedSandbox,
};
use hyperlight_testing::simple_guest_as_string;
use hyperlight_testing::{
    c_callback_guest_as_string, c_simple_guest_as_string, callback_guest_as_string,
};

/// Returns a rust/c simpleguest depending on environment variable GUEST.
/// Uses rust guest by default. Run test with envirnoment variable GUEST="c" to use the c version
/// If a test is only applicable to rust, use `new_uninit_rust`` instead
pub fn new_uninit() -> Result<UninitializedSandbox> {
    UninitializedSandbox::new(
        GuestBinary::FilePath(get_c_or_rust_simpleguest_path()),
        None,
        None,
        None,
    )
}

/// Use this instead of the `new_uninit` if you want your test to only run with the rust guest, not the c guest
pub fn new_uninit_rust() -> Result<UninitializedSandbox> {
    UninitializedSandbox::new(
        GuestBinary::FilePath(simple_guest_as_string().unwrap()),
        None,
        None,
        None,
    )
}

pub fn get_simpleguest_sandboxes<'a>(
    writer: Option<&dyn HostFunction1<String, i32>>, // An optional writer to make sure correct info is passed to the host printer
) -> Vec<MultiUseSandbox<'a>> {
    let path = get_c_or_rust_simpleguest_path();

    #[allow(unused_mut)] // for linux
    let mut res = vec![
        // in hypervisor
        UninitializedSandbox::new(GuestBinary::FilePath(path.clone()), None, None, writer)
            .unwrap()
            .evolve(Noop::default())
            .unwrap(),
    ];

    #[cfg(target_os = "windows")]
    {
        // in process
        res.push(
            UninitializedSandbox::new(
                GuestBinary::FilePath(path.clone()),
                None,
                Some(hyperlight_host::SandboxRunOptions::RunInProcess(false)),
                writer,
            )
            .unwrap()
            .evolve(Noop::default())
            .unwrap(),
        );

        // in memory with loadlib
        res.push(
            UninitializedSandbox::new(
                GuestBinary::FilePath(path.clone()),
                None,
                Some(hyperlight_host::SandboxRunOptions::RunInProcess(true)),
                writer,
            )
            .unwrap()
            .evolve(Noop::default())
            .unwrap(),
        )
    }
    res
}

pub fn get_callbackguest_uninit_sandboxes(
    writer: Option<&dyn HostFunction1<String, i32>>, // An optional writer to make sure correct info is passed to the host printer
) -> Vec<UninitializedSandbox> {
    let path = get_c_or_rust_callbackguest_path();

    #[allow(unused_mut)] // for linux
    let mut res = vec![
        // in hypervisor
        UninitializedSandbox::new(GuestBinary::FilePath(path.clone()), None, None, writer).unwrap(),
    ];

    #[cfg(target_os = "windows")]
    {
        // in process
        res.push(
            UninitializedSandbox::new(
                GuestBinary::FilePath(path.clone()),
                None,
                Some(hyperlight_host::SandboxRunOptions::RunInProcess(false)),
                writer,
            )
            .unwrap(),
        );

        // in memory with loadlib
        res.push(
            UninitializedSandbox::new(
                GuestBinary::FilePath(path.clone()),
                None,
                Some(hyperlight_host::SandboxRunOptions::RunInProcess(true)),
                writer,
            )
            .unwrap(),
        )
    }
    res
}

// returns the GUEST environment variable, or "rust" if not set.
fn get_c_or_rust_simpleguest_path() -> String {
    let guest_type = std::env::var("GUEST").unwrap_or("rust".to_string());
    match guest_type.as_str() {
        "rust" => simple_guest_as_string().unwrap(),
        "c" => c_simple_guest_as_string().unwrap(),
        _ => panic!("Unknown guest type '{guest_type}', use either 'rust' or 'c'"),
    }
}

fn get_c_or_rust_callbackguest_path() -> String {
    let guest_type = std::env::var("GUEST").unwrap_or("rust".to_string());
    match guest_type.as_str() {
        "rust" => callback_guest_as_string().unwrap(),
        "c" => c_callback_guest_as_string().unwrap(),
        _ => panic!("Unknown guest type '{guest_type}', use either 'rust' or 'c'"),
    }
}
