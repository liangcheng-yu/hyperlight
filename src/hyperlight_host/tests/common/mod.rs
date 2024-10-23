use hyperlight_host::func::HostFunction1;
use hyperlight_host::sandbox_state::sandbox::EvolvableSandbox;
use hyperlight_host::sandbox_state::transition::Noop;
use hyperlight_host::{GuestBinary, MultiUseSandbox, Result, UninitializedSandbox};
use hyperlight_testing::{
    c_callback_guest_as_string, c_simple_guest_as_string, callback_guest_as_string,
    simple_guest_as_string,
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

pub fn get_simpleguest_sandboxes(
    writer: Option<&dyn HostFunction1<String, i32>>, // An optional writer to make sure correct info is passed to the host printer
) -> Vec<MultiUseSandbox> {
    let path = get_c_or_rust_simpleguest_path();

    // when env variable GUEST="c", `path`` will be a .exe file already, so we run the same guest twice.
    // This will be fixed when c guests are compiled to elf
    let mut exe_path = path.trim_end_matches(".exe").to_string();
    exe_path.push_str(".exe");

    vec![
        // in hypervisor elf
        UninitializedSandbox::new(GuestBinary::FilePath(path.clone()), None, None, writer)
            .unwrap()
            .evolve(Noop::default())
            .unwrap(),
        // in hypervisor exe
        UninitializedSandbox::new(GuestBinary::FilePath(exe_path.clone()), None, None, writer)
            .unwrap()
            .evolve(Noop::default())
            .unwrap(),
        // in-process elf
        #[cfg(inprocess)]
        UninitializedSandbox::new(
            GuestBinary::FilePath(path.clone()),
            None,
            Some(hyperlight_host::SandboxRunOptions::RunInProcess(false)),
            writer,
        )
        .unwrap()
        .evolve(Noop::default())
        .unwrap(),
        //in-process exe
        #[cfg(inprocess)]
        UninitializedSandbox::new(
            GuestBinary::FilePath(exe_path.clone()),
            None,
            Some(hyperlight_host::SandboxRunOptions::RunInProcess(false)),
            writer,
        )
        .unwrap()
        .evolve(Noop::default())
        .unwrap(),
        // loadlib in process
        #[cfg(all(target_os = "windows", inprocess))]
        UninitializedSandbox::new(
            GuestBinary::FilePath(exe_path.clone()),
            None,
            Some(hyperlight_host::SandboxRunOptions::RunInProcess(true)),
            writer,
        )
        .unwrap()
        .evolve(Noop::default())
        .unwrap(),
    ]
}

pub fn get_callbackguest_uninit_sandboxes(
    writer: Option<&dyn HostFunction1<String, i32>>, // An optional writer to make sure correct info is passed to the host printer
) -> Vec<UninitializedSandbox> {
    let path = get_c_or_rust_callbackguest_path();

    // when env variable GUEST="c", `path`` will be a .exe file already, so we run the same guest twice.
    // This will be fixed when c guests are compiled to elf
    let mut exe_path = path.trim_end_matches(".exe").to_string();
    exe_path.push_str(".exe");

    vec![
        // in hypervisor elf
        UninitializedSandbox::new(GuestBinary::FilePath(path.clone()), None, None, writer).unwrap(),
        // in hypervisor exe
        UninitializedSandbox::new(GuestBinary::FilePath(exe_path.clone()), None, None, writer)
            .unwrap(),
        // in-process elf
        #[cfg(inprocess)]
        UninitializedSandbox::new(
            GuestBinary::FilePath(path.clone()),
            None,
            Some(hyperlight_host::SandboxRunOptions::RunInProcess(false)),
            writer,
        )
        .unwrap(),
        //in-process exe
        #[cfg(inprocess)]
        UninitializedSandbox::new(
            GuestBinary::FilePath(exe_path.clone()),
            None,
            Some(hyperlight_host::SandboxRunOptions::RunInProcess(false)),
            writer,
        )
        .unwrap(),
        // loadlib in process
        #[cfg(all(target_os = "windows", inprocess))]
        UninitializedSandbox::new(
            GuestBinary::FilePath(exe_path.clone()),
            None,
            Some(hyperlight_host::SandboxRunOptions::RunInProcess(true)),
            writer,
        )
        .unwrap(),
    ]
}

// returns the the path of simpleguest binary. Picks rust/c version depending on environment variable GUEST (or rust by default if unset)
pub(crate) fn get_c_or_rust_simpleguest_path() -> String {
    let guest_type = std::env::var("GUEST").unwrap_or("rust".to_string());
    match guest_type.as_str() {
        "rust" => simple_guest_as_string().unwrap(),
        "c" => c_simple_guest_as_string().unwrap(),
        _ => panic!("Unknown guest type '{guest_type}', use either 'rust' or 'c'"),
    }
}

// returns the the path of callbackguest binary. Picks rust/ version depending on environment variable GUEST (or rust by default if unset)
fn get_c_or_rust_callbackguest_path() -> String {
    let guest_type = std::env::var("GUEST").unwrap_or("rust".to_string());
    match guest_type.as_str() {
        "rust" => callback_guest_as_string().unwrap(),
        "c" => c_callback_guest_as_string().unwrap(),
        _ => panic!("Unknown guest type '{guest_type}', use either 'rust' or 'c'"),
    }
}
