// allow ununsed members in here because this is a work in progress
// TODO: remove this once this is no longer a work in progress
#![allow(dead_code)]

use super::surrogate_process::SurrogateProcess;
use anyhow::{anyhow, bail, Result};
use core::ffi::c_void;
use std::ffi::CString;
use std::mem::{size_of, MaybeUninit};
use std::path::Path;
use std::sync::{
    mpsc,
    mpsc::{Receiver, Sender},
    Mutex, Once,
};
use windows::core::{PCSTR, PSTR};
use windows::s;
use windows::Win32::Foundation::{GetLastError, HANDLE};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectA, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_BASIC_LIMIT_INFORMATION,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    CreateProcessA, CREATE_SUSPENDED, PROCESS_INFORMATION, STARTUPINFOA,
};

// This is the name of the surrogate process binary that will be used to create surrogate processes.
// The process does nothing , it just sleeps forever. Its only purpose is to provide a host for memory that will be mapped
// into the guest using the `WHvMapGpaRange2` API.
pub(crate) const SURROGATE_PROCESS_BINARY_NAME: &str = "HyperlightSurrogate.exe";
// The maximum number of surrogate processes that can be created.
// (This is a factor of limitations in the `WHvMapGpaRange2` API which only allows 512 different process handles).
const NUMBER_OF_SURROGATE_PROCESSES: usize = 512;

/// `SurrogateProcessManager` manages HyperlightSurrogate processes. These processes are required to allow multiple WHP Partitions to be created in a single process.
///
/// The documented API WHvMapGpaRange (https://docs.microsoft.com/en-us/virtualization/api/hypervisor-platform/funcs/whvmapgparange) returns an error "Cannot create the partition for the virtualization infrastructure
/// driver because another partition with the same name already exists. (0xC0370008)\ ERROR_VID_PARTITION_ALREADY_EXISTS" when called more than once from a process.
///
/// There is an undocumented API (WHvMapGpaRange2) that has a second parameter which is a handle to a process. This process merely has to exist, the memory being
/// mapped from the host to the virtual machine is allocated/freed  in this process using VirtualAllocEx/VirtualFreeEx. Memory for the HyperVisor partition is copied to and from the host process from/into the surroage process
/// in Sandbox before and after the VCPU is run.
///
/// This struct deals with the creation/destruction of these surrogate processes (HyperlightSurrogate.exe) , pooling of the process handles, the distribution of these handles from the pool to
/// a Hyperlight Sandbox instance and the return of the handle to the pool once a Sandbox instance is destroyed, it also allocates and frees memory in the surrogate process on allocation/return to/from a Sandbox instance.
/// It is intended to be used as a singleton and is thread safe.
///
/// There is a limit of 512 partitions per process therefore this class will create a maximum of 512 processes, and if the pool is empty when a Sandbox is created it will
/// wait for a free process.

pub(crate) struct SurrogateProcessManager {
    job_handle: HANDLE,
    process_receiver: Mutex<Receiver<HANDLE>>,
    process_sender: Sender<HANDLE>,
}

impl SurrogateProcessManager {
    /// Gets a surrogate process from the pool of surrogate processes and allocates memory in the process. This should be called when a new HyperV on Windows Driver is created.
    pub(crate) fn get_surrogate_process(
        &self,
        size: usize,
        memory_address: *const c_void,
    ) -> Result<SurrogateProcess> {
        let process_handle = self
            .process_receiver
            .lock()
            .unwrap()
            .recv()
            .map_err(|e| anyhow!(e.to_string()))?;

        let allocated_address = unsafe {
            VirtualAllocEx(
                process_handle,
                Some(memory_address),
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };

        if allocated_address.is_null() {
            return Err(anyhow!("VirtualAllocEx failed"));
        }

        Ok(SurrogateProcess {
            process_handle,
            allocated_address,
        })
    }
    /// Returns a surrogate process to the pool of surrogate processes and frees memory in the process. This should be called when a sandbox using HyperV on Windows is dropped.
    pub(crate) fn return_surrogate_process(
        &self,
        surrogate_process: SurrogateProcess,
    ) -> Result<()> {
        unsafe {
            if !VirtualFreeEx(
                surrogate_process.process_handle,
                surrogate_process.allocated_address as *mut c_void,
                0,
                MEM_RELEASE,
            )
            .as_bool()
            {
                return Err(anyhow!("VirtualFreeEx failed"));
            }
        }

        self.process_sender
            .clone()
            .send(surrogate_process.process_handle)
            .map_err(|e| anyhow!(e.to_string()))
    }

    // Creates all the surrogate process when the struct is first created.

    fn create_surrogate_processes(
        &self,
        surrogate_process_path: &Path,
        job_handle: &HANDLE,
    ) -> Result<()> {
        for _ in 0..NUMBER_OF_SURROGATE_PROCESSES {
            let surrogate_process = create_surrogate_process(surrogate_process_path, job_handle)?;
            self.process_sender.clone().send(surrogate_process).unwrap();
        }

        Ok(())
    }
}

impl Drop for SurrogateProcessManager {
    fn drop(&mut self) {
        unsafe {
            // Terminating the job object will terminate all the surrogate processes.
            TerminateJobObject(self.job_handle, 0);
        }
    }
}
static mut SURROGATE_PROCESSES_MANAGER: MaybeUninit<SurrogateProcessManager> =
    MaybeUninit::uninit();

/// Gets the singleton SurrogateProcessManager. This should be called when a new HyperV on Windows Driver is created.
pub(crate) fn get_surrogate_process_manager() -> Result<&'static SurrogateProcessManager> {
    static ONCE: Once = Once::new();

    let surrogate_process_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join(SURROGATE_PROCESS_BINARY_NAME);

    if !Path::new(&surrogate_process_path).exists() {
        bail!(
            "get_surrogate_process_manager: file {} does not exist",
            &surrogate_process_path.display()
        );
    }

    ONCE.call_once(|| {
        let (sender, receiver): (Sender<HANDLE>, Receiver<HANDLE>) = mpsc::channel();
        let job_handle = create_job_object().unwrap();
        let surrogate_process_manager = SurrogateProcessManager {
            job_handle,
            process_receiver: Mutex::new(receiver),
            process_sender: sender,
        };

        surrogate_process_manager
            .create_surrogate_processes(&surrogate_process_path, &job_handle)
            .unwrap();

        unsafe {
            SURROGATE_PROCESSES_MANAGER.write(surrogate_process_manager);
        }
    });

    unsafe { Ok(SURROGATE_PROCESSES_MANAGER.assume_init_ref()) }
}

// Creates a job object that will terminate all the surrogate processes when the struct instance is dropped.
fn create_job_object() -> Result<HANDLE> {
    let security_attributes: SECURITY_ATTRIBUTES = Default::default();

    let job_object = unsafe {
        CreateJobObjectA(
            Some(&security_attributes),
            s!("HyperlightSurrogateJobObject"),
        )
        .map_err(|e| anyhow!(e.to_string()))?
    };

    unsafe {
        let mut job_object_information = JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION {
                LimitFlags: JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                ..Default::default()
            },
            ..Default::default()
        };
        let job_object_information_ptr: *mut c_void =
            &mut job_object_information as *mut _ as *mut c_void;
        if !SetInformationJobObject(
            job_object,
            JobObjectExtendedLimitInformation,
            job_object_information_ptr,
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
        .as_bool()
        {
            return Err(anyhow!("SetInformationJobObject failed"));
        }
    };

    Ok(job_object)
}

/// Creates a surrogate process and adds it to the job object.
/// Process is created suspended, its only used as a host for memory
/// the memory is allocated and freed when the process is returned to the pool.
/// The process memory is written to before and read after running the virtual processor in the HyperV partition.
/// All manipulation of the memory is done in memory allocated to the Sandbox which is then copied to and from the surrogate process.

fn create_surrogate_process(surrogate_process_path: &Path, job_handle: &HANDLE) -> Result<HANDLE> {
    let process_handle = unsafe {
        let mut process_information: PROCESS_INFORMATION = std::mem::zeroed();
        let mut startup_info: STARTUPINFOA = std::mem::zeroed();
        let process_attributes: SECURITY_ATTRIBUTES = Default::default();
        let thread_attributes: SECURITY_ATTRIBUTES = Default::default();
        startup_info.cb = std::mem::size_of::<STARTUPINFOA>() as u32;

        // This is painful, there has to be a better way to do this.

        let cmd_line = surrogate_process_path.to_str().unwrap();
        let ccmd_line = CString::new(cmd_line).unwrap().into_raw() as *mut u8;
        let p_cmd_line = PSTR::from_raw(ccmd_line);

        let process_created = CreateProcessA(
            PCSTR::null(),
            p_cmd_line,
            Some(&process_attributes),
            Some(&thread_attributes),
            false,
            CREATE_SUSPENDED,
            None,
            None,
            &startup_info,
            &mut process_information,
        );

        if !process_created.as_bool() {
            return Err(anyhow!("Create Surrogate Process failed"));
        }

        process_information.hProcess
    };

    unsafe {
        if !AssignProcessToJobObject(*job_handle, process_handle).as_bool() {
            let hresult = GetLastError();
            return Err(anyhow!(
                "Assign SurrogateProcess To JobObject Failed: {}",
                hresult.to_hresult()
            ));
        }
    }

    Ok(process_handle)
}
#[cfg(test)]
mod tests {

    use super::*;
    use crate::testing::surrogate_binary::copy_surrogate_exe;
    use rand::{thread_rng, Rng};
    use serial_test::serial;
    use std::ffi::CStr;
    use std::time::Instant;
    use std::{thread, time::Duration};
    use windows::Win32::Foundation::BOOL;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::JobObjects::IsProcessInJob;
    use windows::Win32::System::Memory::{
        VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
    };
    #[test]
    #[serial]
    fn test_surrogate_process_manager() {
        assert!(copy_surrogate_exe());

        let mut threads = Vec::new();
        // create more threads than surrogate processes as we want to test that the manager can handle multiple threads requesting processes at the same time when there are not enough processes available.
        for t in 0..NUMBER_OF_SURROGATE_PROCESSES * 2 {
            let thread_handle = thread::spawn(move || -> Result<()> {
                let surrogate_process_manager = get_surrogate_process_manager();
                let mut rng = thread_rng();
                let size = 4096;
                assert!(surrogate_process_manager.is_ok());
                let surrogate_process_manager = surrogate_process_manager.unwrap();
                let job_handle = surrogate_process_manager.job_handle;
                for p in 0..NUMBER_OF_SURROGATE_PROCESSES {
                    let allocated_address = unsafe {
                        VirtualAlloc(None, size, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE)
                    };
                    let timer = Instant::now();
                    let surrogate_process =
                        surrogate_process_manager.get_surrogate_process(1024, allocated_address);
                    let elapsed = timer.elapsed();
                    if let Err(e) = surrogate_process {
                        println!("Get Error Thread {} Process {}: {:?}", t, p, e);
                        return Err(e);
                    }
                    // Print out the time it took to get the process if its greater than 150ms (this is just to allow us to see that threads are blocking on the process queue)
                    if (elapsed.as_millis() as u64) > 150 {
                        println!("Get Process Time Thread {} Process {}: {:?}", t, p, elapsed);
                    }
                    let surrogate_process = surrogate_process.unwrap();
                    let mut result: BOOL = Default::default();
                    unsafe {
                        assert!(IsProcessInJob(
                            surrogate_process.process_handle,
                            job_handle,
                            &mut result
                        )
                        .as_bool());
                        assert!(result.as_bool());
                    }
                    // in real use the process will not get returned immediately
                    let n: u64 = rng.gen_range(1..16);
                    thread::sleep(Duration::from_millis(n));
                    if let Err(e) =
                        surrogate_process_manager.return_surrogate_process(surrogate_process)
                    {
                        println!("Return Error Thread {} Process {}: {:?}", t, p, e);
                        return Err(e);
                    }
                    unsafe {
                        VirtualFree(allocated_address, 0, MEM_RELEASE);
                    }
                }
                Ok(())
            });
            threads.push(thread_handle);
        }

        for thread_handle in threads {
            assert!(thread_handle.join().is_ok());
        }

        assert_number_of_surrogate_processes(NUMBER_OF_SURROGATE_PROCESSES);
        unsafe { SURROGATE_PROCESSES_MANAGER.assume_init_drop() };
        assert_number_of_surrogate_processes(0);
    }

    fn assert_number_of_surrogate_processes(expected_count: usize) {
        let sleep_count = 10;
        loop {
            let snapshot_handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
            assert!(snapshot_handle.is_ok());
            let snapshot_handle = snapshot_handle.unwrap();
            let mut process_entry = PROCESSENTRY32 {
                dwSize: size_of::<PROCESSENTRY32>() as u32,
                ..Default::default()
            };
            let mut result =
                unsafe { Process32First(snapshot_handle, &mut process_entry).as_bool() };
            let mut count = 0;
            while result {
                if let Ok(process_name) = unsafe {
                    CStr::from_ptr(process_entry.szExeFile.as_ptr() as *const i8).to_str()
                } {
                    if process_name == SURROGATE_PROCESS_BINARY_NAME {
                        count += 1;
                    }
                }
                unsafe {
                    result = Process32Next(snapshot_handle, &mut process_entry).as_bool();
                }
            }

            // if the expected count is 0, we are waiting for the processes to exit, this doesnt happen immediately, so we wait for a bit

            if (expected_count == 0) && (count > 0) && (sleep_count < 30) {
                thread::sleep(Duration::from_secs(1));
            } else {
                assert_eq!(count, expected_count);
                break;
            }
        }
    }
}
