using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using Hyperlight.Native;
using Microsoft.Win32.SafeHandles;

namespace Hyperlight.Hypervisors
{
    internal class HyperV : Hypervisor, IDisposable
    {
        readonly HyperVSurrogateProcessManager processManager = HyperVSurrogateProcessManager.Instance;
        readonly SurrogateProcess surrogateProcess;
        bool disposedValue;
        readonly IntPtr hPartition = IntPtr.Zero;
        readonly WindowsHypervisorPlatform.WHV_REGISTER_NAME[] registerNames;
        readonly WindowsHypervisorPlatform.MyUInt128[] registerValues;
        readonly WindowsHypervisorPlatform.WHV_REGISTER_NAME[] ripName = new WindowsHypervisorPlatform.WHV_REGISTER_NAME[] { WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRip };
        readonly WindowsHypervisorPlatform.MyUInt128[] ripValue = new WindowsHypervisorPlatform.MyUInt128[1];
        readonly bool virtualProcessorCreated;
        readonly ulong size;

        internal HyperV(IntPtr sourceAddress, int pml4_addr, ulong size, ulong entryPoint, ulong rsp, Action<ushort, byte> outb) : base(sourceAddress, entryPoint, rsp, outb)
        {
            this.size = size;
            WindowsHypervisorPlatform.WHvCreatePartition(out hPartition);
            WindowsHypervisorPlatform.SetProcessorCount(hPartition, 1);
            WindowsHypervisorPlatform.WHvSetupPartition(hPartition);
            surrogateProcess = processManager.GetProcess((IntPtr)size, sourceAddress);
            var hProcess = surrogateProcess.safeProcessHandle.DangerousGetHandle();
            WindowsHypervisorPlatform.WHvMapGpaRange2(hPartition, hProcess, sourceAddress, (IntPtr)0x200000/*IntPtr.Zero*/, size, WindowsHypervisorPlatform.WHV_MAP_GPA_RANGE_FLAGS.WHvMapGpaRangeFlagRead | WindowsHypervisorPlatform.WHV_MAP_GPA_RANGE_FLAGS.WHvMapGpaRangeFlagWrite | WindowsHypervisorPlatform.WHV_MAP_GPA_RANGE_FLAGS.WHvMapGpaRangeFlagExecute);
            WindowsHypervisorPlatform.WHvCreateVirtualProcessor(hPartition, 0, 0);
            virtualProcessorCreated = true;

            var registerNamesList = new List<WindowsHypervisorPlatform.WHV_REGISTER_NAME>();
            var registerValuesList = new List<WindowsHypervisorPlatform.MyUInt128>();
            void AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME registerName, ulong registerValueLow, ulong registerValueHigh)
            {
                registerNamesList.Add(registerName);
                registerValuesList.Add(new WindowsHypervisorPlatform.MyUInt128() { low = registerValueLow, high = registerValueHigh });
            }

            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterCr3, (ulong)pml4_addr, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterCr4, X64.CR4_PAE | X64.CR4_OSFXSR | X64.CR4_OSXMMEXCPT, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterCr0, X64.CR0_PE | X64.CR0_MP | X64.CR0_ET | X64.CR0_NE | X64.CR0_WP | X64.CR0_AM | X64.CR0_PG, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterEfer, X64.EFER_LME | X64.EFER_LMA, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterCs, 0, 0xa09b0008ffffffff);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterDs, 0, 0xa0930010ffffffff);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterEs, 0, 0xa0930010ffffffff);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterFs, 0, 0xa0930010ffffffff);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterGs, 0, 0/*0xa0930010ffffffff*/);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterSs, 0, 0xa0930010ffffffff);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRflags, 0x0002, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRip, entryPoint, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRsp, rsp, 0);

            // MSVC entry point takes first three integer arguments in rcx, rdx, r8
            // GCC entry point takes first three integer arguments in rdi, rsi, rdx
            // We always pass MSVC style assuming that _start is compiled with __attribute__((msabi))
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRcx, 1, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRdx, 1, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterR8, 1, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterR9, 1, 0);

            // Here is the GCC way
            //AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRdi, 1, 0);
            //AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRsi, 1, 0);
            //AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRdx, 1, 0);

            registerNames = registerNamesList.ToArray();
            registerValues = registerValuesList.ToArray();
        }

        internal override void DispactchCallFromHost(ulong pDispatchFunction)
        {
            // Move rip to the DispatchFunction pointer
            ripValue[0].low = pDispatchFunction;
            WindowsHypervisorPlatform.WHvSetVirtualProcessorRegisters(hPartition, 0, ripName, 1, ripValue);
            ExecuteUntilHalt();
        }

        internal override void ExecuteUntilHalt()
        {
            WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_CONTEXT exitContext;
            do
            {
                // TODO optimise this
                // the following write to and read from process memory is required as we need to use
                // surrogate processes to allow more than one WHP Partition per process
                // see HyperVSurrogateProcessManager
                // this needs updating so that 
                // 1. it only writes to memory that changes between usage
                // 2. memory is allocated in the process once and then only freed and reallocated if the 
                // memory needs to grow.
                if (!OS.WriteProcessMemory(surrogateProcess.safeProcessHandle.DangerousGetHandle(), surrogateProcess.sourceAddress, sourceAddress, (IntPtr)size, out IntPtr written))
                {
                    int error = Marshal.GetLastWin32Error();
                    if (error != 0)
                    {
                        throw new ApplicationException($"WriteProcessMemory Error: {error}");
                    }
                }
                WindowsHypervisorPlatform.WHvRunVirtualProcessor(hPartition, 0, out exitContext, (uint)Marshal.SizeOf<WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_CONTEXT>());
                if (!OS.ReadProcessMemory(surrogateProcess.safeProcessHandle.DangerousGetHandle(), surrogateProcess.sourceAddress, sourceAddress, (IntPtr)size, out IntPtr read))
                {
                    int error = Marshal.GetLastWin32Error();
                    if (error != 0)
                    {
                        throw new ApplicationException($"WriteProcessMemory Error: {error}");
                    }
                }

                if (exitContext.ExitReason == WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_REASON.WHvRunVpExitReasonX64IoPortAccess)
                {
                    HandleOutb(exitContext.IoPortAccess.PortNumber, 0/*todo add value*/);

                    // Move rip forward to next instruction (size of current instruction in lower byte of InstructionLength_Cr8_Reserved)
                    ripValue[0].low = exitContext.VpContext.Rip + (ulong)(exitContext.VpContext.InstructionLength_Cr8_Reserved & 0xF);
                    WindowsHypervisorPlatform.WHvSetVirtualProcessorRegisters(hPartition, 0, ripName, 1, ripValue);
                }
                else if (exitContext.ExitReason != WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_REASON.WHvRunVpExitReasonX64Halt)
                {
                    var v2 = new WindowsHypervisorPlatform.MyUInt128[registerNames.Length];
                    WindowsHypervisorPlatform.WHvGetVirtualProcessorRegisters(hPartition, 0, registerNames, (uint)registerNames.Length, v2);
                    for (var i = 0; i < v2.Length; i++)
                    {
                        //TODO: put this in an exception.
                        Console.WriteLine($"{registerNames[i]} - {v2[i].low:X16}");
                    }
                    throw new Exception($"Did not recieve a halt as expected - Received {exitContext.ExitReason}");
                }
            } while (exitContext.ExitReason != WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_REASON.WHvRunVpExitReasonX64Halt);
        }

        internal override void Run(int argument1, int argument2, int argument3)
        {
            {
                registerValues[^4].low = (ulong)0x200000;
                registerValues[^3].low = (ulong)argument1;
                registerValues[^2].low = (ulong)argument2;
                registerValues[^1].low = (ulong)argument3;

                WindowsHypervisorPlatform.WHvSetVirtualProcessorRegisters(hPartition, 0, registerNames, (uint)registerNames.Length, registerValues);
            }

            ExecuteUntilHalt();
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    processManager.ReturnProcess(surrogateProcess);
                }

                if (virtualProcessorCreated)
                {
                    // TODO: Handle error
                    _ = WindowsHypervisorPlatform.WHvDeleteVirtualProcessor(hPartition, 0);
                }

                if (IntPtr.Zero != hPartition)
                {
                    WindowsHypervisorPlatform.WHvDeletePartition(hPartition);
                }

                disposedValue = true;
            }
        }

        ~HyperV()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: false);
        }

        public override void Dispose()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }
    }
}
