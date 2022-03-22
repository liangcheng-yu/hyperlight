using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Runtime.InteropServices;
using Hyperlight.Native;
using Microsoft.Win32.SafeHandles;

namespace Hyperlight.Hypervisors
{
    internal class SurrogateProcess
    {
        public SafeProcessHandle safeProcessHandle;
        public IntPtr sourceAddress;
    }

    /// <summary>
    /// This class manages HyperlightSurrogate processes.    /// These processes are required to allow multiple WHP Partitions to be created in a single process.
    /// 
    /// The documented API WHvMapGpaRange (https://docs.microsoft.com/en-us/virtualization/api/hypervisor-platform/funcs/whvmapgparange) returns an error 
    /// Cannot create the partition for the virtualization infrastructure driver because another partition with the same name already exists. (0xC0370008)\ ERROR_VID_PARTITION_ALREADY_EXISTS
    /// when called for a second time from a process.
    /// 
    /// There is an undocumented API (WHvMapGpaRange2) that has a second parameter which is a handle to a process. This process merely has to exist, the memeory being 
    /// mapped from the host to the guest partition is allocated/freed using VirtualAllocEx/VirtualFreeEx. Memory for the HyperVisor partition is copied to and from the host process in Sandbox before and after the VCPU is run.
    /// 
    /// This class deals with the creation/ destruction of these surrogate processes (HyperlightSurrogate.exe) , pooling of the process handles, the distribution of these handles from the pool to 
    /// a Hyperlight Sandbox instance and the return of the handle to the pool once a Sandbox instance is disposed, it also allocates and frees memory in the process on allocation/return to/from a Sandbox instance.
    /// It is intended to be created as a singleton and assigned to a static property in the HyperV class.
    /// 
    /// There is a limit of 512 partitions per process therefore this class will create a maximum of 512 processes, and if the pool is empty when a Sandbox is created it will 
    /// wait for a free process, this behaviour can be overridden by passing a cancellation token to the GetProcess method. 
    /// </summary>
    /// 

    internal sealed class HyperVSurrogateProcessManager : IDisposable
    {
        private const string SurrogateProcessBinaryName = "HyperlightSurrogate.exe";
        private static readonly Lazy<HyperVSurrogateProcessManager> instance = new(() => new HyperVSurrogateProcessManager());
        // The maximum number of processes that can be created 
        internal const int NumberOfProcesses = 512;
        // A job is used to make sure that the processes are cleaned up when the host process ends
        IntPtr jobHandle = IntPtr.Zero;
        private bool disposedValue;
        private readonly BlockingCollection<SafeProcessHandle> surrogateProcesses = new(NumberOfProcesses);
        public static HyperVSurrogateProcessManager Instance => instance.Value;

        private HyperVSurrogateProcessManager()
        {
            CreateJobObject();
            CreateProcesses();
        }

        /// <summary>
        /// Creates a job object which each process is added to , this job object ensures that all processes are terminated when the host ends
        /// </summary>
        private void CreateJobObject()
        {
            var securityAttributes = new OS.SecurityAttributes();
            jobHandle = OS.CreateJobObject(securityAttributes, "HyperlightSurrogateJob");

            var jobLimitInfo = new OS.JobBasicLimitInfo
            {
                LimitFlags = OS.JobLimitInfo.JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            };
            var extendedJobInfo = new OS.JobExtentedLimitInfo
            {
                BasicLimitInformation = jobLimitInfo
            };

            var length = Marshal.SizeOf<OS.JobExtentedLimitInfo>();
            var extendedInfoPtr = Marshal.AllocHGlobal(length);
            Marshal.StructureToPtr<OS.JobExtentedLimitInfo>(extendedJobInfo, extendedInfoPtr, false);
            OS.SetInformationJobObject(jobHandle, OS.JobObjectInfoType.ExtendedLimitInformation, extendedInfoPtr, (uint)length);
            Marshal.FreeHGlobal(extendedInfoPtr);
        }
        /// <summary>
        /// Process is created suspended, its only used as a host for memory 
        /// the memory is allocated and freed when the process is used/released by a sandbox
        /// See GetProcess and Return Process below
        /// The process memory is written to before and read after running the virtual processor in the HyperV partition.
        /// All manipulation of the memory is done in memory allocated to the Sandbox whih is then copied to and from the surrogate process.
        /// </summary>
        private void CreateProcesses()
        {
            var surrogateBinaryPath = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, SurrogateProcessBinaryName);
            for (var i = 0; i < NumberOfProcesses; i++)
            {
                var startupInfo = new OS.StartupInfo();
                var securityAttributes = new OS.SecurityAttributes();
                OS.CreateProcess(null, surrogateBinaryPath, securityAttributes, securityAttributes, false, OS.CreateProcessFlags.CREATE_SUSPENDED, IntPtr.Zero, null, startupInfo, out OS.ProcessInformation pi);
                var error = Marshal.GetLastWin32Error();
                if (error != 0)
                {
                    throw new ApplicationException("Failed to create Hyperlight Surrogate process");
                }

                var safeProcessHandle = new SafeProcessHandle(pi.hProcess, true);
                OS.AssignProcessToJobObject(jobHandle, pi.hProcess);
                surrogateProcesses.Add(safeProcessHandle);
            }
        }
        // Allocates a process from the pool to a Sandbox instance so that the process can be used in call to WHvMapGpaRange2
        // and allocates Virtual Memory in that process to match the size and address of the memory of the Sandbox instance.
        //
        internal SurrogateProcess GetProcess(IntPtr size, IntPtr sourceAddress, CancellationToken cancellationToken=default(CancellationToken))
        {
            var safeProcessHandle = surrogateProcesses.Take(cancellationToken);
            var destAddress = OS.VirtualAllocEx(safeProcessHandle.DangerousGetHandle(), sourceAddress, size, OS.AllocationType.Commit | OS.AllocationType.Reserve, OS.MemoryProtection.EXECUTE_READWRITE);
            return new SurrogateProcess { safeProcessHandle = safeProcessHandle, sourceAddress = destAddress };
        }
        // returns the process to the pool . this is called when a Sandbox is disposed. Also free the virtual memory allocated to the process.
        internal void ReturnProcess(SurrogateProcess surrogateProcess)
        {
            //TODO: HandleError
            _= OS.VirtualFreeEx(surrogateProcess.safeProcessHandle.DangerousGetHandle(), surrogateProcess.sourceAddress, IntPtr.Zero, (uint)OS.AllocationType.Release);
            surrogateProcesses.Add(surrogateProcess.safeProcessHandle);
        }

        private void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    foreach (var safeProcessHandle in surrogateProcesses)
                    {
                        safeProcessHandle.Dispose();
                    }
                    surrogateProcesses.Dispose();
                    if (jobHandle != IntPtr.Zero)
                    {
                        OS.CloseHandle(jobHandle);
                    }
                }
                disposedValue = true;
            }
        }

        public void Dispose()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }
    }
}
