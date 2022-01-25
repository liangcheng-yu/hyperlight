using System;
using System.IO;
using System.Runtime.InteropServices;
using Hyperlight.HyperVisors;
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
        static bool IsWindows => RuntimeInformation.IsOSPlatform(OSPlatform.Windows);
        static bool IsLinux => RuntimeInformation.IsOSPlatform(OSPlatform.Linux);
        public static bool IsSupportedPlatform => IsLinux || IsWindows;
        Hypervisor hyperVisor;
        IntPtr sourceAddress = IntPtr.Zero;
        readonly ulong size;
        readonly string guestBinaryPath;
        IntPtr loadAddress = IntPtr.Zero;
        readonly bool recycleAfterRun;
        readonly byte[] initialMemorySavedForMultipleRunCalls;
        readonly bool runFromProcessMemory;
        readonly bool runFromGuestBinary;
        ulong entryPoint;
        ulong rsp;
        readonly HyperlightGuestInterfaceGlue guestInterfaceGlue;
        private bool disposedValue; // To detect redundant calls
        delegate long CallMain(int a, int b, int c);

        // Platform dependent delegate for callbacks from native code when native code is calling 'outb' functionality
        // On Linux, delegates passed from .NET core to native code expect arguments to be passed RDI, RSI, RDX, RCX.
        // On Winodws, the expected order starts with RCX, RDX.  Our native code assumes this Windows calling convention
        // so 'port' is passed in RCX and 'value' is passed in RDX.  When run in Linux, we have an alternate callback
        // that will take RCX and RDX in the different positions and pass it to the HandleOutb method correctly
        delegate void CallOutb_Windows(ushort port, byte value);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        delegate void CallOutb_Linux(int unused1, int unused2, byte value, ushort port);
        delegate void CallDispatchFunction();
        int countRunCalls;

        public Sandbox(ulong size, string guestBinaryPath) : this(size, guestBinaryPath, SandboxRunOptions.None, null)
        {
        }

        public Sandbox(ulong size, string guestBinaryPath, object instanceOrType) : this(size, guestBinaryPath, SandboxRunOptions.None, instanceOrType)
        {
        }

        public Sandbox(ulong size, string guestBinaryPath, SandboxRunOptions runOptions) : this(size, guestBinaryPath, runOptions, null)
        {
        }

        public Sandbox(ulong size, string guestBinaryPath, SandboxRunOptions runOptions, object instanceOrType)
        {
            if (!IsSupportedPlatform)
            {
                throw new PlatformNotSupportedException("Hyperlight is not supported on this platform");
            }

            if (!File.Exists(guestBinaryPath))
            {
                throw new Exception($"Cannot find file {guestBinaryPath} to load into hyperlight");
            }

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
                throw new Exception("Cannot run from guest binary and recycle after run at the same time");
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
                    throw new Exception("Hypervisor not found");
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
            // Get DispatchFunction pointer from PEB
            var pDispatchFunction = (ulong)Marshal.ReadInt64((IntPtr)0x204008);

            var headerSize = 0x08 + 0x08 + 0x08 * args.Length; // Pointer to function name, count of args, and arg list
            var stringTable = new SimpleStringTable((IntPtr)0x220000 + headerSize, 0x10000 - headerSize);

            Marshal.WriteInt64((IntPtr)0x220000, (long)stringTable.AddString(functionName));
            Marshal.WriteInt64((IntPtr)0x220008, args.Length);
            for (var i = 0; i < args.Length; i++)
            {
                if (args[i].GetType() == typeof(int))
                    Marshal.WriteInt64((IntPtr)(0x220010 + 8 * i), (int)args[i]);
                else if (args[i].GetType() == typeof(string))
                    Marshal.WriteInt64((IntPtr)(0x220010 + 8 * i), (long)(0x8000000000000000 | stringTable.AddString((string)args[i])));
                else
                    throw new Exception("Unsupported parameter type");
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

            return Marshal.ReadInt32((IntPtr)0x220000);
        }

        void LoadGuestBinary()
        {
            if (runFromGuestBinary)
            {
                if (!IsWindows)
                {
                    // If not on Windows runFromBinary doesn't mean anything because we cannot use LoadLibrary.
                    throw new NotImplementedException("RunFromBinary is only supported on Windows");
                }

                loadAddress = OS.LoadLibrary(guestBinaryPath);
                if (loadAddress != (IntPtr)0x230000)
                {
                    throw new Exception("Wrong base address");
                }

                // Mark first byte as '0' so we know we are running in hyperlight VM and not as real windows exe
                OS.VirtualProtect(loadAddress, (UIntPtr)(1024 * 4), OS.MemoryProtection.EXECUTE_READWRITE, out _);
                Marshal.WriteByte(loadAddress, (byte)'J');
                var e_lfanew = Marshal.ReadInt32((IntPtr)0x230000 + 0x3C);
                var entryPointFileOffset = (uint)Marshal.ReadInt32((IntPtr)0x230000 + e_lfanew + 0x28);
                entryPoint += 0x230000 + entryPointFileOffset; // Currently entryPoint points to the VA of the start of the file

                // Allocate only the first 0x30000 bytes - After that is the EXE
                sourceAddress = OS.VirtualAlloc((IntPtr)0x200000/*IntPtr.Zero*/, (IntPtr)0x30000, OS.AllocationType.Commit | OS.AllocationType.Reserve, OS.MemoryProtection.EXECUTE_READWRITE);
                if (IntPtr.Zero == sourceAddress)
                {
                    throw new Exception("VirtualAlloc failed");
                }

            }
            else
            {
                var payload = File.ReadAllBytes(guestBinaryPath);
                // Load the binary into memory
                entryPoint = 0x230000;
                if ((payload[0] == (byte)'M' || payload[0] == (byte)'J') && payload[1] == (byte)'Z')
                {
                    payload[0] = (byte)'J'; // Mark first byte as '0' so we know we are running in hyperlight VM and not as real windows exe
                    var e_lfanew = BitConverter.ToInt32(payload, 0x3C);
                    var entryPointFileOffset = (uint)BitConverter.ToInt32(payload, e_lfanew + 0x28);
                    entryPoint += entryPointFileOffset; // Currently entryPoint points to the VA of the start of the file
                }

                sourceAddress = OS.Allocate((IntPtr)0x200000, size);
                Marshal.Copy(payload, 0, sourceAddress + 0x30000, payload.Length);
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
                        throw new Exception("Only int return types are supported");
                    }

                    var parameterSignature = "";
                    foreach (var pi in mi.GetParameters())
                    {
                        if (pi.ParameterType == typeof(int))
                            parameterSignature += "i";
                        else if (pi.ParameterType == typeof(string))
                            parameterSignature += "$";
                        else
                            throw new Exception("Only int and string parameters are supported");
                    }

                    peb.AddFunction(mi.Name, $"({parameterSignature})i", 0);
                }
            }
            peb.WriteToMemory(IntPtr.Add(sourceAddress, 0x4000), 0x1000);
        }

        public void SetUpHyperVisorPartition()
        {
            rsp = size + 0x200000; // Add 0x200000 because that's the start of mapped memory

            // For MSVC, move rsp down by 0x28.  This gives the called 'main' function the appearance that rsp was
            // was 16 byte aligned before the 'call' that calls main (note we don't really have a return value on the
            // stack but some assembly instructions are expecting rsp have started 0x8 bytes off of 16 byte alignment
            // when 'main' is invoked.  We do 0x28 instead of 0x8 because MSVC can expect that there are 0x20 bytes
            // of space to write to by the called function.  I am not sure if this happens with the 'main' method, but
            // we do this just in case.
            // NOTE: We do this also for GCC freestanding binaries because we specify __attribute__((ms_abi)) on the start method
            rsp -= 0x28;

            // Create pagetable
            var pml4_addr = 0x201000;
            var pdpt_addr = 0x202000;
            var pd_addr = 0x203000;
            var pml4 = IntPtr.Add(sourceAddress, pml4_addr - 0x200000);
            var pdpt = IntPtr.Add(sourceAddress, pdpt_addr - 0x200000);
            var pd = IntPtr.Add(sourceAddress, pd_addr - 0x200000);

            Marshal.WriteInt64(pml4, 0, (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | (ulong)pdpt_addr));
            Marshal.WriteInt64(pdpt, 0, (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | (ulong)pd_addr));

            for (var i = 0 /*We do not map first 2 megs*/; i < 512; i++)
            {
                Marshal.WriteInt64(IntPtr.Add(pd, i * 8), ((i /*We map each VA to physical memory 2 megs lower*/) << 21) + (long)(X64.PDE64_PRESENT | X64.PDE64_RW | X64.PDE64_USER | X64.PDE64_PS));
            }

            if (IsLinux)
            {
                hyperVisor = new KVM(pml4_addr, size, entryPoint, rsp, HandleOutb);
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
                throw new Exception("You must set option RecycleAfterRun when creating the Sandbox if you need to call Run more than once");
            }

            if (recycleAfterRun)
            {
                Marshal.Copy(initialMemorySavedForMultipleRunCalls!, 0, sourceAddress, (int)size);
            }

            if (workloadBytes != null && workloadBytes.Length > 0)
            {
                Marshal.Copy(workloadBytes, 0, sourceAddress + 0x10000, workloadBytes.Length);
            }

            if (runFromProcessMemory)
            {
                var callMain = Marshal.GetDelegateForFunctionPointer<CallMain>((IntPtr)entryPoint);
                if (IsLinux)
                {
                    Marshal.WriteInt64((IntPtr)0x210000 - 16, (long)Marshal.GetFunctionPointerForDelegate<CallOutb_Linux>((_, _, value, port) => HandleOutb(port, value)));
                }
                else if (IsWindows)
                {
                    Marshal.WriteInt64((IntPtr)0x210000 - 16, (long)Marshal.GetFunctionPointerForDelegate<CallOutb_Windows>((port, value) => HandleOutb(port, value)));
                }
                else
                {
                    // Should never get here
                    throw new NotSupportedException();
                }

                returnValue = callMain(argument1, argument2, argument3);

            }
            else
            {
                // We do not currently look at returnValue - It will be stored at 0x220000
                hyperVisor!.Run(argument1, argument2, argument3);
            }
            countRunCalls++;

            return ((uint)Marshal.ReadInt32(sourceAddress + 0x20000), Marshal.PtrToStringAnsi(sourceAddress + 0x20010), Marshal.ReadInt64(sourceAddress + 0x20008));
        }

        internal void HandleOutb(ushort port, byte _)
        {
            switch (port)
            {

                case 101: // call Function
                    {
                        var strPtr = Marshal.ReadInt64((IntPtr)0x220000);
                        var functionName = Marshal.PtrToStringAnsi((IntPtr)strPtr);
                        if (string.IsNullOrEmpty(functionName))
                        {
                            throw new Exception("Function name is null or empty");
                        }

                        if (guestInterfaceGlue != null)
                        {
                            if (!guestInterfaceGlue.mapHostFunctionNamesToMethodInfo.ContainsKey(functionName))
                            {
                                throw new Exception($"Could not find host function name {functionName}");
                            }
                            var mi = guestInterfaceGlue.mapHostFunctionNamesToMethodInfo[functionName];
                            var parameters = mi.GetParameters();
                            var args = new object[parameters.Length];
                            for (var i = 0; i < parameters.Length; i++)
                            {
                                if (parameters[i].ParameterType == typeof(int))
                                {
                                    args[i] = Marshal.ReadInt32((IntPtr)(0x220008 + 8 * i));
                                }
                                else if (parameters[i].ParameterType == typeof(string))
                                {
                                    args[i] = Marshal.PtrToStringAnsi(Marshal.ReadIntPtr((IntPtr)(0x220008 + 8 * i)));
                                }
                                else
                                {
                                    throw new Exception("Unsupported parameter type");
                                }
                            }
                            var returnFromHost = (int)guestInterfaceGlue.DispatchCallFromGuest(functionName, args);
                            Marshal.WriteInt32((IntPtr)0x210000, returnFromHost);
                        }
                        else
                        {
                            throw new Exception($"Could not find host function name {functionName}. GuestInterfaceGlue is null");
                        }
                        break;
                    }
                case 100: // Write with no carriage return
                    {
                        // Read string from 0x220000;
                        var str = Marshal.PtrToStringAnsi(sourceAddress + 0x20000);
                        var oldColor = Console.ForegroundColor;
                        Console.ForegroundColor = ConsoleColor.Green;
                        Console.Write(str);
                        Console.ForegroundColor = oldColor;
                        break;
                    }
                case 99: // Write with carriage return
                    {
                        // Read string from 0x220000;
                        var str = Marshal.PtrToStringAnsi(sourceAddress + 0x20000);
                        var oldColor = Console.ForegroundColor;
                        Console.ForegroundColor = ConsoleColor.Green;
                        Console.WriteLine(str);
                        Console.ForegroundColor = oldColor;
                        break;
                    }
                case 98:
                    {
                        var a = Marshal.ReadInt32(sourceAddress + 0x20000);
                        var b = Marshal.ReadInt32(sourceAddress + 0x20004);
                        var digits = a.ToString().Length + b.ToString().Length;
                        Marshal.WriteInt32(sourceAddress + 0x20000, digits);
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

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    hyperVisor?.Dispose();
                }

                if (IntPtr.Zero != sourceAddress)
                {
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
