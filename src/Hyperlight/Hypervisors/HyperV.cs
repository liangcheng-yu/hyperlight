using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
using Hyperlight.Core;
using Hyperlight.Native;
using Hyperlight.Wrapper;


namespace Hyperlight.Hypervisors
{
    internal sealed class HyperV : Hypervisor, IDisposable
    {
        readonly SurrogateProcess surrogateProcess;
        bool disposedValue;
        readonly IntPtr hPartition = IntPtr.Zero;
        readonly WindowsHypervisorPlatform.WHV_REGISTER_NAME[] registerNames;
        readonly WindowsHypervisorPlatform.MyUInt128[] registerValues;
        readonly WindowsHypervisorPlatform.WHV_REGISTER_NAME[] ripName = new WindowsHypervisorPlatform.WHV_REGISTER_NAME[] { WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRip };
        readonly WindowsHypervisorPlatform.MyUInt128[] ripValue = new WindowsHypervisorPlatform.MyUInt128[1];
        readonly bool virtualProcessorCreated;
        readonly ulong size;

        internal HyperV(IntPtr sourceAddress, ulong pml4_addr, ulong size, ulong entryPoint, ulong rsp, Action<ushort, byte> outb, Action handleMemoryAccess) : base(sourceAddress, entryPoint, rsp, outb, handleMemoryAccess)
        {
            this.size = size;
            WindowsHypervisorPlatform.WHvCreatePartition(out hPartition);
            WindowsHypervisorPlatform.SetProcessorCount(hPartition, 1);
            WindowsHypervisorPlatform.WHvSetupPartition(hPartition);
            surrogateProcess = HyperVSurrogateProcessManager.Instance.GetProcess((IntPtr)size, sourceAddress);
            var hProcess = surrogateProcess.SafeProcessHandle.DangerousGetHandle();
            // sourceAddress points to first guard page, so we offset by 1 page
            sourceAddress += (int)OS.GetPageSize();
            // size is size including guard pages, so we subtract 2 * 4096
            size -= 2 * OS.GetPageSize();
            WindowsHypervisorPlatform.WHvMapGpaRange2(hPartition, hProcess, sourceAddress, (IntPtr)SandboxMemoryLayout.BaseAddress, size, WindowsHypervisorPlatform.WHV_MAP_GPA_RANGE_FLAGS.WHvMapGpaRangeFlagRead | WindowsHypervisorPlatform.WHV_MAP_GPA_RANGE_FLAGS.WHvMapGpaRangeFlagWrite | WindowsHypervisorPlatform.WHV_MAP_GPA_RANGE_FLAGS.WHvMapGpaRangeFlagExecute);
            WindowsHypervisorPlatform.WHvCreateVirtualProcessor(hPartition, 0, 0);
            virtualProcessorCreated = true;

            var registerNamesList = new List<WindowsHypervisorPlatform.WHV_REGISTER_NAME>();
            var registerValuesList = new List<WindowsHypervisorPlatform.MyUInt128>();
            void AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME registerName, ulong registerValueLow, ulong registerValueHigh)
            {
                registerNamesList.Add(registerName);
                registerValuesList.Add(new WindowsHypervisorPlatform.MyUInt128() { low = registerValueLow, high = registerValueHigh });
            }

            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterCr3, pml4_addr, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterCr4, X64.CR4_PAE | X64.CR4_OSFXSR | X64.CR4_OSXMMEXCPT, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterCr0, X64.CR0_PE | X64.CR0_MP | X64.CR0_ET | X64.CR0_NE | X64.CR0_WP | X64.CR0_AM | X64.CR0_PG, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterEfer, X64.EFER_LME | X64.EFER_LMA, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterCs, 0, 0xa09b0008ffffffff);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRflags, 0x0002, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRip, entryPoint, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRsp, rsp, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterR9, 0, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterR8, 0, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRdx, 0, 0);
            AddRegister(WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRcx, 0, 0);

            registerNames = registerNamesList.ToArray();
            registerValues = registerValuesList.ToArray();
        }

        internal override void DispatchCallFromHost(ulong pDispatchFunction)
        {
            // Move rip to the DispatchFunction pointer
            ripValue[0].low = pDispatchFunction;
            WindowsHypervisorPlatform.WHvSetVirtualProcessorRegisters(hPartition, 0, ripName, 1, ripValue);
            ExecuteUntilHalt();
        }

        private void ExecuteUntilHalt()
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
                if (!OS.WriteProcessMemory(surrogateProcess.SafeProcessHandle.DangerousGetHandle(), surrogateProcess.SourceAddress, sourceAddress, (IntPtr)size, out IntPtr written))
                {
                    int error = Marshal.GetLastWin32Error();
                    if (error != 0)
                    {
                        HyperlightException.LogAndThrowException($"WriteProcessMemory Error: {error}", GetType().Name);
                    }
                }
                WindowsHypervisorPlatform.WHvRunVirtualProcessor(hPartition, 0, out exitContext, (uint)Marshal.SizeOf<WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_CONTEXT>());
                if (!OS.ReadProcessMemory(surrogateProcess.SafeProcessHandle.DangerousGetHandle(), surrogateProcess.SourceAddress, sourceAddress, (IntPtr)size, out IntPtr read))
                {
                    int error = Marshal.GetLastWin32Error();
                    if (error != 0)
                    {
                        HyperlightException.LogAndThrowException($"ReadProcessMemory Error: {error}", GetType().Name);
                    }
                }

                switch (exitContext.ExitReason)
                {
                    case WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_REASON.WHvRunVpExitReasonX64IoPortAccess:
                        {
                            // get rax as byte
                            var rax = exitContext.IoPortAccess.Rax;
                            var value = (byte)(rax & 0xFF);

                            HandleOutb(exitContext.IoPortAccess.PortNumber, value);

                            // Move rip forward to next instruction (size of current instruction in lower byte of InstructionLength_Cr8_Reserved)

                            ripValue[0].low = exitContext.VpContext.Rip + (ulong)(exitContext.VpContext.InstructionLength_Cr8_Reserved & 0xF);
                            WindowsHypervisorPlatform.WHvSetVirtualProcessorRegisters(hPartition, 0, ripName, 1, ripValue);
                            break;
                        }
                    case WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_REASON.WHvRunVpExitReasonX64Halt:
                        {
                            break;
                        }
                    case WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_REASON.WHvRunVpExitReasonMemoryAccess:
                        {
                            HandleMemoryAccess();
                            ThrowExitException(exitContext);
                            break;
                        }
                    default:
                        {
                            ThrowExitException(exitContext);
                            break;
                        }
                }

            } while (exitContext.ExitReason != WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_REASON.WHvRunVpExitReasonX64Halt);
        }

        private void ThrowExitException(WindowsHypervisorPlatform.WHV_RUN_VP_EXIT_CONTEXT exitContext)
        {
            var v2 = new WindowsHypervisorPlatform.MyUInt128[registerNames.Length];
            WindowsHypervisorPlatform.WHvGetVirtualProcessorRegisters(hPartition, 0, registerNames, (uint)registerNames.Length, v2);
            StringBuilder context = new($"Did not receive a halt from Hypervisor as expected - Received {exitContext.ExitReason}");
            for (var i = 0; i < v2.Length; i++)
            {
#pragma warning disable CA1305 // Specify IFormatProvider
                context.AppendLine($"{registerNames[i]} - {v2[i].low:X16}");
#pragma warning restore CA1305 // Specify IFormatProvider
            }
            HyperlightException.LogAndThrowException(context.ToString(), GetType().Name);
        }

        internal override void ResetRSP(ulong rsp)
        {
            var rspValue = new WindowsHypervisorPlatform.MyUInt128[1];
            var rspName = new WindowsHypervisorPlatform.WHV_REGISTER_NAME[] { WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRsp };
            rspValue[0].low = rsp;
            WindowsHypervisorPlatform.WHvSetVirtualProcessorRegisters(hPartition, 0, rspName, 1, rspValue);
        }

        internal override void Initialise(IntPtr pebAddress, ulong seed, uint pageSize)
        {
            Debug.Assert(registerNames[^1] == WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRcx);
            registerValues[^1].low = (ulong)pebAddress;
            Debug.Assert(registerNames[^2] == WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterRdx);
            registerValues[^2].low = seed;
            Debug.Assert(registerNames[^3] == WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterR8);
            registerValues[^3].low = pageSize;
            // The fourth parameter is the default log level for the rust guest we will just set it to error (1) for now
            Debug.Assert(registerNames[^4] == WindowsHypervisorPlatform.WHV_REGISTER_NAME.WHvX64RegisterR9);
            registerValues[^4].low = 1;
            WindowsHypervisorPlatform.WHvSetVirtualProcessorRegisters(hPartition, 0, registerNames, (uint)registerNames.Length, registerValues);
            ExecuteUntilHalt();
        }

        private void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    HyperVSurrogateProcessManager.Instance.ReturnProcess(surrogateProcess);
                }

                if (virtualProcessorCreated)
                {
                    Syscall.CheckReturnVal(
                        "HyperV WHvDeleteVirtualProcessor",
                        () => WindowsHypervisorPlatform.WHvDeleteVirtualProcessor(hPartition, 0),
                        0
                    );
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
