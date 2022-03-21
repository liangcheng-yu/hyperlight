using System;
using System.Collections.Concurrent;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using Hyperlight.Hypervisors;
using Hyperlight.Native;

namespace Hyperlight
{
    // Address Space Layout - Assume 10 meg physical (0xA00000) memory and 1 meg code (0x100000)
    // Physical      Virtual
    // 0x00000000    0x00200000    Start of physical/min-Valid virtual
    // 0x00001000    0x00201000    PML4
    // 0x00002000    0x00202000    PDTP
    // 0x00003000    0x00203000    PD
    // 0x00010000    0x00210000    64k for input data
    // 0x00020000    0x00220000    64k for output data
    // 0x00030000    0x00230000    Start of Code
    // 0x0012FFFF    0x0032FFFF    End of code (Start of code + 0x100000-1)
    // 0x009FFFFF    0x00BFFFFF    End of physical/max-Valid virtual
    // 0x00A00000    0x00C00000    Starting RSP

    // Address Space Layout - Assume max 0x3FE00000 physical memory and 1 meg code (0x100000)
    // Physical      Virtual
    // 0x00000000    0x00200000    Start of physical/min-Valid virtual
    // 0x00001000    0x00201000    PML4
    // 0x00002000    0x00202000    PDTP
    // 0x00003000    0x00203000    PD
    // 0x00010000    0x00210000    64k for input data
    // 0x00020000    0x00220000    64k for output data
    // 0x00030000    0x00230000    Start of Code
    // 0x0012FFFF    0x0032FFFF    End of code (Start of code + 0x100000-1)
    // 0x3FDFFFFF    0x3FFFFFFF    End of physical/max-Valid virtual
    // 0x3FE00000    0x40000000    Starting RSP

    // For our page table, we only mapped virtual memory up to 0x3FFFFFFF and map each 2 meg 
    // virtual chunk to physical addresses 2 megabytes below the virtual address.  Since we
    // map virtual up to 0x3FFFFFFF, the max physical address we handle is 0x3FDFFFFF (or 
    // 0x3FEF0000 physical total memory)

    [Flags]
    public enum SandboxRunOptions
    {
        None = 0,
        RunInProcess = 1,
        RecycleAfterRun = 2,
        RunFromGuestBinary = 4,
    }
    public class Sandbox : IDisposable
    {
        static object peInfoLock = new object();
        static readonly ConcurrentDictionary<string, PEInfo> guestPEInfo = new(StringComparer.InvariantCultureIgnoreCase);
        static bool IsWindows => RuntimeInformation.IsOSPlatform(OSPlatform.Windows);
        static bool IsLinux => RuntimeInformation.IsOSPlatform(OSPlatform.Linux);
        public static bool IsSupportedPlatform => IsLinux || IsWindows;
        Hypervisor hyperVisor;
        IntPtr sourceAddress = IntPtr.Zero;
        readonly ulong size;
        readonly string guestBinaryPath;
        IntPtr loadAddress = IntPtr.Zero;

        readonly IntPtr baseAddress = (IntPtr)0x200000;
        readonly IntPtr codeAddress = (IntPtr)0x230000;
        readonly int pCodeOffset = 0x10000 - 24;
        readonly int pOutBOffset = 0x10000 - 16;
        readonly int dispatchPointerOffset = 0x4008;
        readonly int inputDataOffset = 0x10000;
        readonly int outputDataOffset = 0x20000;
        readonly int codeOffset = 0x30000;
        readonly int pml4_addr = 0x201000;
        readonly int pdpt_addr = 0x202000;
        readonly int pd_addr = 0x203000;
        readonly bool recycleAfterRun;
        readonly byte[] initialMemorySavedForMultipleRunCalls;
        readonly bool runFromProcessMemory;
        readonly bool runFromGuestBinary;
        bool didRunFromGuestBinary;
        const int IS_RUNNING_FROM_GUEST_BINARY = 1;
        static int isRunningFromGuestBinary = 0;
        readonly StringWriter writer;
        ulong entryPoint;
        ulong rsp;
        readonly HyperlightGuestInterfaceGlue guestInterfaceGlue;
        private bool disposedValue; // To detect redundant calls
        delegate long CallLinuxEntryPoint(int c, int b, int a, IntPtr baseAddress);
        delegate long CallWindowsEntryPoint(IntPtr baseAddress, int a, int b, int c);

        unsafe delegate* unmanaged<IntPtr, int, int, int, long> callEntryPointWindows;
        unsafe delegate* unmanaged<int, int, int, IntPtr, long> callEntryPointLinux;

        // Platform dependent delegate for callbacks from native code when native code is calling 'outb' functionality
        // On Linux, delegates passed from .NET core to native code expect arguments to be passed RDI, RSI, RDX, RCX.
        // On Windows, the expected order starts with RCX, RDX.  Our native code assumes this Windows calling convention
        // so 'port' is passed in RCX and 'value' is passed in RDX.  When run in Linux, we have an alternate callback
        // that will take RCX and RDX in the different positions and pass it to the HandleOutb method correctly

        delegate void CallOutb_Windows(ushort port, byte value);
        delegate void CallOutb_Linux(int unused1, int unused2, byte value, ushort port);
        delegate void CallDispatchFunction();
        int countRunCalls;

        public Sandbox(ulong size, string guestBinaryPath, object instanceOrType) : this(size, guestBinaryPath, null, instanceOrType)
        {
        }

        public Sandbox(ulong size, string guestBinaryPath, StringWriter writer = null) : this(size, guestBinaryPath, SandboxRunOptions.None, null, writer)
        {
        }

        public Sandbox(ulong size, string guestBinaryPath, StringWriter writer, object instanceOrType = null) : this(size, guestBinaryPath, SandboxRunOptions.None, instanceOrType, writer)
        {
        }

        public Sandbox(ulong size, string guestBinaryPath, SandboxRunOptions runOptions, StringWriter writer = null) : this(size, guestBinaryPath, runOptions, null, writer)
        {
        }

        public Sandbox(ulong size, string guestBinaryPath, SandboxRunOptions runOptions, object instanceOrType, StringWriter writer = null)
        {
            if (!IsSupportedPlatform)
            {
                throw new PlatformNotSupportedException("Hyperlight is not supported on this platform");
            }

            if (!File.Exists(guestBinaryPath))
            {
                throw new ArgumentException($"Cannot find file {guestBinaryPath} to load into hyperlight");
            }
            this.writer = writer;
            this.guestBinaryPath = guestBinaryPath;
            // TODO: Validate the size.
            this.size = size;
            this.recycleAfterRun = (runOptions & SandboxRunOptions.RecycleAfterRun) == SandboxRunOptions.RecycleAfterRun;
            this.runFromProcessMemory = (runOptions & SandboxRunOptions.RunInProcess) == SandboxRunOptions.RunInProcess ||
                                        (runOptions & SandboxRunOptions.RunFromGuestBinary) == SandboxRunOptions.RunFromGuestBinary;
            this.runFromGuestBinary = (runOptions & SandboxRunOptions.RunFromGuestBinary) == SandboxRunOptions.RunFromGuestBinary;

            // TODO: should we make this work?
            if (recycleAfterRun && runFromGuestBinary)
            {
                throw new ArgumentException("Cannot run from guest binary and recycle after run at the same time");
            }

            if (null != instanceOrType)
            {
                this.guestInterfaceGlue = new HyperlightGuestInterfaceGlue(instanceOrType, this);
            }

            LoadGuestBinary();
            SetUpHyperLightPEB();

            // If we are NOT running from process memory, we have to setup a Hypervisor partition
            if (!runFromProcessMemory)
            {
                if (IsHypervisorPresent())
                {
                    SetUpHyperVisorPartition();
                }
                else
                {
                    throw new ArgumentException("Hypervisor not found");
                }
            }

            if (recycleAfterRun)
            {
                initialMemorySavedForMultipleRunCalls = new byte[size];
                Marshal.Copy(sourceAddress, initialMemorySavedForMultipleRunCalls, 0, (int)size);
            }

        }

        internal object DispatchCallFromHost(string functionName, object[] args)
        {

            ulong offset = 0;
            if (!runFromGuestBinary && !runFromProcessMemory)
            {
                offset = (ulong)sourceAddress - (ulong)baseAddress;
            }

            var outputDataAddress = sourceAddress + outputDataOffset;
            var dispatchFunctionAddress = sourceAddress + dispatchPointerOffset;
            // Get DispatchFunction pointer from PEB
            var pDispatchFunction = (ulong)Marshal.ReadInt64(dispatchFunctionAddress);

            var headerSize = 0x08 + 0x08 + 0x08 * args.Length; // Pointer to function name, count of args, and arg list
            var stringTable = new SimpleStringTable(outputDataAddress + headerSize, inputDataOffset - headerSize, offset);

            Marshal.WriteInt64(outputDataAddress, (long)stringTable.AddString(functionName));
            Marshal.WriteInt64(outputDataAddress + 0x8, args.Length);
            for (var i = 0; i < args.Length; i++)
            {
                if (args[i].GetType() == typeof(int))
                {
                    Marshal.WriteInt64(outputDataAddress + 0x10 + 8 * i, (int)args[i]);
                }
                else if (args[i].GetType() == typeof(string))
                {
                    var addr = (long)(0x8000000000000000 | stringTable.AddString((string)args[i]));
                    Marshal.WriteInt64(outputDataAddress + 0x10 + 8 * i, addr);
                }
                else
                {
                    throw new ArgumentException("Unsupported parameter type");
                }
            }

            if (runFromProcessMemory)
            {
                var callDispatchFunction = Marshal.GetDelegateForFunctionPointer<CallDispatchFunction>((IntPtr)pDispatchFunction);
                callDispatchFunction();
            }
            else
            {
                hyperVisor!.DispactchCallFromHost(pDispatchFunction);
            }

            return Marshal.ReadInt32(outputDataAddress);
        }

        void LoadGuestBinary()
        {
            var peInfo = guestPEInfo.GetOrAdd(guestBinaryPath, (guestBinaryPath) => GetPEInfo(guestBinaryPath, (ulong)codeAddress));

            if (runFromGuestBinary)
            {
                if (!IsWindows)
                {
                    // If not on Windows runFromBinary doesn't mean anything because we cannot use LoadLibrary.
                    throw new NotImplementedException("RunFromBinary is only supported on Windows");
                }

                // LoadLibrary does not support multple independent instances of a binary beng loaded 
                // so we cannot support multiple instances using loadlibrary

                if (Interlocked.CompareExchange(ref isRunningFromGuestBinary, IS_RUNNING_FROM_GUEST_BINARY, 0) == 0)
                {
                    didRunFromGuestBinary = true;
                }
                else
                {
                    throw new ApplicationException("Only one instance of Sandbox is allowed when running from guest binary");
                }

                loadAddress = OS.LoadLibrary(guestBinaryPath);

                // Mark first byte as 'J' so we know we are running in hyperlight VM and not as real windows exe
                // TODO: protect memory again after modification
                OS.VirtualProtect(loadAddress, (UIntPtr)(1024 * 4), OS.MemoryProtection.EXECUTE_READWRITE, out _);
                Marshal.WriteByte(loadAddress, (byte)'J');
                var e_lfanew = Marshal.ReadInt32(loadAddress + 0x3C);

                entryPoint += (ulong)loadAddress + peInfo.EntryPointOffset; // Currently entryPoint points to the VA of the start of the file

                // Allocate 0x30001 for IO the extra byte at the end is where the code will be loaded InProcess or under HyperVisor
                // Allows the guest to find the code if we are debugging 
                sourceAddress = OS.Allocate((IntPtr)0, (ulong)codeOffset + 2);

                if (IntPtr.Zero == sourceAddress)
                {
                    throw new ApplicationException("VirtualAlloc failed");
                }

                // Write a pointer to code so that guest exe can check that it is running in Hyperlight

                Marshal.WriteInt64(sourceAddress + pCodeOffset, (long)loadAddress);
            }
            else
            {

                sourceAddress = OS.Allocate((IntPtr)0, size);

                // If we are running in memory the entry point will be relative to the sourceAddress if we are running in a Hypervisor it will be relative to 0x230000 which is where the code is loaded in the GP
                if (runFromProcessMemory)
                {
                    entryPoint = (ulong)sourceAddress + (ulong)codeOffset + peInfo.EntryPointOffset;
                    Marshal.Copy(peInfo.Payload, 0, sourceAddress + codeOffset, peInfo.Payload.Length);

                    // When loading in memory we need to fix up the relocations in the exe to reflect the address the exe was loaded at.
                    peInfo.PatchExeRelocations((ulong)sourceAddress + (ulong)codeOffset);
                }
                else
                {
                    entryPoint = (ulong)codeAddress + peInfo.EntryPointOffset;
                    Marshal.Copy(peInfo.HyperVisorPayload, 0, sourceAddress + codeOffset, peInfo.Payload.Length);
                }
            }
        }

        void SetUpHyperLightPEB()
        {
            var peb = new HyperlightPEB();
            if (guestInterfaceGlue != null)
            {
                foreach (var mi in guestInterfaceGlue.mapHostFunctionNamesToMethodInfo.Values)
                {
                    // TODO: Add support for void return types
                    if (mi.ReturnType != typeof(int))
                    {
                        throw new ArgumentException("Only int return types are supported");
                    }

                    var parameterSignature = "";
                    foreach (var pi in mi.GetParameters())
                    {
                        if (pi.ParameterType == typeof(int))
                            parameterSignature += "i";
                        else if (pi.ParameterType == typeof(string))
                            parameterSignature += "$";
                        else
                            throw new ArgumentException("Only int and string parameters are supported");
                    }

                    peb.AddFunction(mi.Name, $"({parameterSignature})i", 0);
                }
            }
            peb.WriteToMemory(IntPtr.Add(sourceAddress, 0x4000), 0x1000);
        }

        public void SetUpHyperVisorPartition()
        {
            rsp = size + (ulong)baseAddress; // Add 0x200000 because that's the start of mapped memory

            // For MSVC, move rsp down by 0x28.  This gives the called 'main' function the appearance that rsp was
            // was 16 byte aligned before the 'call' that calls main (note we don't really have a return value on the
            // stack but some assembly instructions are expecting rsp have started 0x8 bytes off of 16 byte alignment
            // when 'main' is invoked.  We do 0x28 instead of 0x8 because MSVC can expect that there are 0x20 bytes
            // of space to write to by the called function.  I am not sure if this happens with the 'main' method, but
            // we do this just in case.
            // NOTE: We do this also for GCC freestanding binaries because we specify __attribute__((ms_abi)) on the start method
            rsp -= 0x28;

            // Create pagetable

            var pml4 = IntPtr.Add(sourceAddress, pml4_addr - (int)baseAddress);
            var pdpt = IntPtr.Add(sourceAddress, pdpt_addr - (int)baseAddress);
            var pd = IntPtr.Add(sourceAddress, pd_addr - (int)baseAddress);

            Marshal.WriteInt64(pml4, 0, (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | (ulong)pdpt_addr));
            Marshal.WriteInt64(pdpt, 0, (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | (ulong)pd_addr));

            for (var i = 0 /*We do not map first 2 megs*/; i < 512; i++)
            {
                Marshal.WriteInt64(IntPtr.Add(pd, i * 8), ((i /*We map each VA to physical memory 2 megs lower*/) << 21) + (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | X64.PDE64_PS));
            }

            if (IsLinux)
            {
                hyperVisor = new KVM(sourceAddress, pml4_addr, size, entryPoint, rsp, HandleOutb);
            }
            else if (IsWindows)
            {
                hyperVisor = new HyperV(sourceAddress, pml4_addr, size, entryPoint, rsp, HandleOutb);
            }
            else
            {
                // Should never get here
                throw new NotSupportedException();
            }
        }

        public (uint returnValue, string returnValueString, long returnValue64) Run(byte[] workloadBytes, int argument1, int argument2, int argument3)
        {
            long returnValue = 0;

            if (countRunCalls > 0 && !recycleAfterRun)
            {
                throw new ArgumentException("You must set option RecycleAfterRun when creating the Sandbox if you need to call Run more than once");
            }

            if (recycleAfterRun)
            {
                Marshal.Copy(initialMemorySavedForMultipleRunCalls!, 0, sourceAddress, (int)size);
            }

            if (workloadBytes != null && workloadBytes.Length > 0)
            {
                Marshal.Copy(workloadBytes, 0, sourceAddress + inputDataOffset, workloadBytes.Length);
            }

            if (runFromProcessMemory)
            {
                if (IsLinux)
                {
                    // This code is unstable, it causes segmetation faults so for now we are throwing an exception if we try to run in process in Linux
                    // I think this is due to the fact that the guest binary is built for windows
                    // x64 compilation for windows uses fastcall which is different on windows and linux
                    // dotnet will default to the calling convention for the platform that the code is running on
                    // so we need to set the calling convention to the one that the guest binary is built for (windows x64 https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention?view=msvc-170)
                    // on linux however, this isn't possible (https://docs.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.callingconvention?view=net-6.0 )
                    // Alternatives:
                    // 1. we need to build the binary for windows and linux and then run the correct version for the platform that we are running on
                    // 2. alter the calling convention of the guest binary and then tell dotnet to use that calling convention
                    // the only option for this seems to be vectorcall https://docs.microsoft.com/en-us/cpp/cpp/vectorcall?view=msvc-170 (cdecl and stdcall are not possible using CL on x64 platform))    
                    // vectorcall is not supported by dotnet  (https://github.com/dotnet/runtime/issues/8300) 
                    // 3. write our own code to correct the calling convention
                    // 4. write epilog/prolog code in the guest binary.     
                    // also see https://www.agner.org/optimize/calling_conventions.pdf
                    // and https://eli.thegreenplace.net/2011/09/06/stack-frame-layout-on-x86-64/
                    // 

                    throw new NotSupportedException("Cannot run in process on Linux");

                    Marshal.WriteInt64(sourceAddress + pOutBOffset, (long)Marshal.GetFunctionPointerForDelegate<CallOutb_Linux>((_, _, value, port) => HandleOutb(port, value)));
                    unsafe
                    {
                        callEntryPointLinux = (delegate* unmanaged<int, int, int, IntPtr, long>)entryPoint;
                        returnValue = callEntryPointLinux(argument1, argument2, argument3, sourceAddress);
                    }
                }
                else if (IsWindows)
                {
                    Marshal.WriteInt64(sourceAddress + pOutBOffset, (long)Marshal.GetFunctionPointerForDelegate<CallOutb_Windows>((port, value) => HandleOutb(port, value)));
                    unsafe
                    {
                        callEntryPointWindows = (delegate* unmanaged<IntPtr, int, int, int, long>)entryPoint;
                        returnValue = callEntryPointWindows(sourceAddress, argument1, argument2, argument3);
                    }
                }
                else
                {
                    // Should never get here
                    throw new NotSupportedException();
                }

            }
            else
            {
                // We do not currently look at returnValue - It will be stored at sourceAddress + outputDataOffset
                hyperVisor!.Run(argument1, argument2, argument3);
            }
            countRunCalls++;

            return ((uint)Marshal.ReadInt32(sourceAddress + outputDataOffset), Marshal.PtrToStringAnsi(sourceAddress + outputDataOffset + 0x10), Marshal.ReadInt64(sourceAddress + outputDataOffset + 0x8));
        }


        //TODO: throwing exceptions here does not work as this function is invoked from native code
        //need to figure out how to return errors and log issues instead

        internal void HandleOutb(ushort port, byte _)
        {
            // Offset contains the adjustment that needs to be made to addresses when running in Hypervisor so that the address reflects the host or guest address correctly
            ulong offset = 0;
            if (!runFromGuestBinary && !runFromProcessMemory)
            {
                offset = (ulong)sourceAddress - (ulong)baseAddress;
            }
            switch (port)
            {

                case 101: // call Function
                    {
                        var outputDataAddress = sourceAddress + outputDataOffset;
                        var strPtr = Marshal.ReadInt64((IntPtr)outputDataAddress);
                        var functionName = Marshal.PtrToStringAnsi((IntPtr)((ulong)strPtr + offset));
                        if (string.IsNullOrEmpty(functionName))
                        {
                            throw new ArgumentNullException("Function name is null or empty");
                        }

                        if (guestInterfaceGlue != null)
                        {
                            if (!guestInterfaceGlue.mapHostFunctionNamesToMethodInfo.ContainsKey(functionName))
                            {
                                throw new ArgumentNullException($"Could not find host function name {functionName}");
                            }
                            var mi = guestInterfaceGlue.mapHostFunctionNamesToMethodInfo[functionName];
                            var parameters = mi.GetParameters();
                            var args = new object[parameters.Length];
                            for (var i = 0; i < parameters.Length; i++)
                            {
                                if (parameters[i].ParameterType == typeof(int))
                                {
                                    args[i] = Marshal.ReadInt32(outputDataAddress + 8 * (i + 1));
                                }
                                else if (parameters[i].ParameterType == typeof(string))
                                {
                                    strPtr = Marshal.ReadInt64(outputDataAddress + 8 * (i + 1));
                                    args[i] = Marshal.PtrToStringAnsi((IntPtr)((ulong)strPtr + offset));
                                }
                                else
                                {
                                    throw new ArgumentException("Unsupported parameter type");
                                }
                            }
                            var returnFromHost = (int)guestInterfaceGlue.DispatchCallFromGuest(functionName, args);
                            Marshal.WriteInt32(sourceAddress + inputDataOffset, returnFromHost);
                        }
                        else
                        {
                            throw new ArgumentNullException($"Could not find host function name {functionName}. GuestInterfaceGlue is null");
                        }
                        break;
                    }
                case 100: // Write with no carriage return
                    {
                        // Read string from 0x20000 offset into virtual memory;
                        var str = Marshal.PtrToStringAnsi(sourceAddress + outputDataOffset);
                        if (this.writer != null)
                        {
                            writer.Write(str);
                        }
                        else
                        {
                            var oldColor = Console.ForegroundColor;
                            Console.ForegroundColor = ConsoleColor.Green;
                            Console.Write(str);
                            Console.ForegroundColor = oldColor;
                        }
                        break;
                    }
                case 99: // Write with carriage return
                    {
                        // Read string from 0x20000 offset into virtual memory;
                        var str = Marshal.PtrToStringAnsi(sourceAddress + outputDataOffset);
                        if (this.writer != null)
                        {
                            writer.WriteLine(str);
                        }
                        else
                        {
                            var oldColor = Console.ForegroundColor;
                            Console.ForegroundColor = ConsoleColor.Green;
                            Console.WriteLine(str);
                            Console.ForegroundColor = oldColor;
                        }
                        break;
                    }
            }
        }

        public static bool IsHypervisorPresent()
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                return LinuxKVM.IsHypervisorPresent();
            }
            else if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                return WindowsHypervisorPlatform.IsHypervisorPresent();
            }
            return false;
        }

        static PEInfo GetPEInfo(string fileName, ulong hyperVisorCodeAddress)
        {
            lock (peInfoLock)
            {
                if (guestPEInfo.ContainsKey(fileName))
                {
                    return guestPEInfo[fileName];
                }
                return new PEInfo(fileName, hyperVisorCodeAddress);
            }
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    if (didRunFromGuestBinary)
                    {
                        Interlocked.Decrement(ref isRunningFromGuestBinary);
                    }

                    hyperVisor?.Dispose();

                }

                if (IntPtr.Zero != sourceAddress)
                {
                    // TODO: check if this should take account of space used by loadlibrary.
                    OS.Free(sourceAddress, size);
                }

                if (IntPtr.Zero != loadAddress)
                {
                    OS.FreeLibrary(loadAddress);
                }

                disposedValue = true;
            }
        }

        ~Sandbox()
        {
            // Do not change this code. Put cleanup code in Dispose(bool disposing) above.
            Dispose(false);
        }

        public void Dispose()
        {
            // Do not change this code. Put cleanup code in Dispose(bool disposing) above.
            Dispose(true);
            GC.SuppressFinalize(this);
        }
    }
}
