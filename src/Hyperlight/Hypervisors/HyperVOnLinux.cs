using System;
using System.Collections.Generic;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Core;
using Hyperlight.Native;
using Hyperlight.Wrapper;

namespace Hyperlight.Hypervisors
{
    internal class HyperVOnLinux : Hypervisor, IDisposable
    {
        private bool disposedValue;
        readonly Context context;
        readonly Handle mshvHandle;
        readonly Handle vmHandle;
        readonly Handle vcpuHandle;
        readonly Handle memoryHandle;
        readonly List<LinuxHyperV.MSHV_REGISTER> registerList = new();
        LinuxHyperV.MSHV_REGISTER[]? registers;
        readonly LinuxHyperV.MSHV_REGISTER[] ripRegister = new LinuxHyperV.MSHV_REGISTER[1] { new() { Name = LinuxHyperV.HV_X64_REGISTER_RIP, Value = new LinuxHyperV.MSHV_U128 { HighPart = 0, LowPart = 0 } } };

        internal HyperVOnLinux(IntPtr sourceAddress, int pml4_addr, ulong size, ulong entryPoint, ulong rsp, Action<ushort, byte> outb, Action handleMemoryAccess) : base(sourceAddress, entryPoint, rsp, outb, handleMemoryAccess)
        {

            if (!LinuxHyperV.IsHypervisorPresent())
            {
                HyperlightException.LogAndThrowException<HyperVOnLinuxException>("HyperV Not Present", Sandbox.CorrelationId.Value!, GetType().Name);
            }

            context = new Context();
            var mshv = LinuxHyperV.open_mshv(context.ctx, LinuxHyperV.REQUIRE_STABLE_API);
            mshvHandle = new Handle(context, mshv);
            if (mshvHandle.IsError())
            {
                HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Unable to open mshv  Error: {mshvHandle.GetErrorMessage()}", Sandbox.CorrelationId.Value!, GetType().Name);
            }

            var vmfd = LinuxHyperV.create_vm(context.ctx, mshvHandle.handle);
            vmHandle = new Handle(context, vmfd);
            if (vmHandle.IsError())
            {
                HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Unable to create HyperV VM Error: {vmHandle.GetErrorMessage()}", Sandbox.CorrelationId.Value!, GetType().Name);
            }

            var vcpu = LinuxHyperV.create_vcpu(context.ctx, vmHandle.handle);
            vcpuHandle = new Handle(context, vcpu);
            if (vcpuHandle.IsError())
            {
                HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Unable to create HyperV VCPU Error:  {vcpuHandle.GetErrorMessage()}", Sandbox.CorrelationId.Value!, GetType().Name);
            }
            var userMemoryRegion = LinuxHyperV.map_vm_memory_region(context.ctx, vmHandle.handle, (ulong)SandboxMemoryLayout.BaseAddress >> 12, (ulong)sourceAddress, size);
            memoryHandle = new Handle(context, userMemoryRegion);
            if (memoryHandle.IsError())
            {
                HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Failed to map User Memory Region Error: {memoryHandle.GetErrorMessage()}", Sandbox.CorrelationId.Value!, GetType().Name);
            }

            AddRegister(LinuxHyperV.HV_X64_REGISTER_CR3, (ulong)pml4_addr, 0);
            AddRegister(LinuxHyperV.HV_X64_REGISTER_CR4, X64.CR4_PAE | X64.CR4_OSFXSR | X64.CR4_OSXMMEXCPT, 0);
            AddRegister(LinuxHyperV.HV_X64_REGISTER_CR0, X64.CR0_PE | X64.CR0_MP | X64.CR0_ET | X64.CR0_NE | X64.CR0_WP | X64.CR0_AM | X64.CR0_PG, 0);
            AddRegister(LinuxHyperV.HV_X64_REGISTER_EFER, X64.EFER_LME | X64.EFER_LMA, 0);
            AddRegister(LinuxHyperV.HV_X64_REGISTER_CS, 0, 0xa09b0008ffffffff);
            AddRegister(LinuxHyperV.HV_X64_REGISTER_RFLAGS, 0x0002, 0);
            AddRegister(LinuxHyperV.HV_X64_REGISTER_RIP, entryPoint, 0);
            AddRegister(LinuxHyperV.HV_X64_REGISTER_RSP, rsp, 0);
        }

        void AddRegister(uint registerName, ulong lowPart, ulong highPart)
        {
            registerList.Add(new LinuxHyperV.MSHV_REGISTER
            {
                Name = registerName,
                Value = new LinuxHyperV.MSHV_U128
                {
                    LowPart = lowPart,
                    HighPart = highPart
                }
            });
        }

        internal override void DispatchCallFromHost(ulong pDispatchFunction)
        {
            ripRegister[0].Value.LowPart = pDispatchFunction;
            var setRegister = LinuxHyperV.set_registers(context.ctx, vcpuHandle.handle, ripRegister, (UIntPtr)ripRegister.Length);
            using (var registerHandle = new Handle(context, setRegister))
            {
                if (registerHandle.IsError())
                {
                    HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Failed setting RIP Error: {registerHandle.GetErrorMessage()}", Sandbox.CorrelationId.Value!, GetType().Name);
                }
            }
            ExecuteUntilHalt();
        }

        internal override void ResetRSP(ulong rsp)
        {
            var reg = new LinuxHyperV.MSHV_REGISTER[1] { new() { Name = LinuxHyperV.HV_X64_REGISTER_RIP, Value = new LinuxHyperV.MSHV_U128 { HighPart = rsp, LowPart = 0 } } };
            var setRegister = LinuxHyperV.set_registers(context.ctx, vcpuHandle.handle, reg, (UIntPtr)reg.Length);
            using (var registerHandle = new Handle(context, setRegister))
            {
                if (registerHandle.IsError())
                {
                    HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Failed setting RSP Error: {registerHandle.GetErrorMessage()}", Sandbox.CorrelationId.Value!, GetType().Name);
                }
            }
        }

        internal override void ExecuteUntilHalt()
        {
            var runResultPtr = IntPtr.Zero;
            LinuxHyperV.MSHV_RUN_MESSAGE runResult;
            try
            {
                do
                {
                    if (runResultPtr != IntPtr.Zero)
                    {
                        LinuxHyperV.free_run_result(runResultPtr);
                        runResultPtr = IntPtr.Zero;
                    }

                    var runResultHdl = LinuxHyperV.run_vcpu(context.ctx, vcpuHandle.handle);
                    using (var runResultHandle = new Handle(context, runResultHdl))
                    {
                        if (runResultHandle.IsError())
                        {
                            HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Failed to run VCPU Error: {runResultHandle.GetErrorMessage()}", Sandbox.CorrelationId.Value!, GetType().Name);
                        }

                        runResultPtr = LinuxHyperV.get_run_result_from_handle(context.ctx, runResultHandle.handle);
                        if (runResultPtr == IntPtr.Zero)
                        {
                            HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Failed to get run result", Sandbox.CorrelationId.Value!, GetType().Name);

                        }

                        runResult = Marshal.PtrToStructure<LinuxHyperV.MSHV_RUN_MESSAGE>(runResultPtr);
                        switch (runResult.MessageType)
                        {
                            case LinuxHyperV.HV_MESSAGE_TYPE_HVMSG_X64_HALT:
                                break;
                            case LinuxHyperV.HV_MESSAGE_TYPE_HVMSG_X64_IO_PORT_INTERCEPT:
                                HandleOutb(runResult.PortNumber, Convert.ToByte(runResult.RAX));
                                ripRegister[0].Value.LowPart = runResult.RIP + runResult.InstructionLength;
                                var setRegister = LinuxHyperV.set_registers(context.ctx, vcpuHandle.handle, ripRegister, (UIntPtr)ripRegister.Length);
                                using (var registerHandle = new Handle(context, setRegister))
                                {
                                    if (registerHandle.IsError())
                                    {
                                        HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Failed setting RIP Error: {registerHandle.GetErrorMessage()}", Sandbox.CorrelationId.Value!, GetType().Name);
                                    }
                                }
                                break;
                            case LinuxHyperV.HV_MESSAGE_TYPE_HVMSG_UNMAPPED_GPA:
                                HandleMemoryAccess();
                                ThrowExitException(runResult.MessageType);
                                break;
                            default:
                                ThrowExitException(runResult.MessageType);
                                break;
                        }

                    }
                }
                while (runResult.MessageType != LinuxHyperV.HV_MESSAGE_TYPE_HVMSG_X64_HALT);
            }
            finally
            {
                if (runResultPtr != IntPtr.Zero)
                {
                    LinuxHyperV.free_run_result(runResultPtr);
                }
            }
        }

        private static void ThrowExitException(uint exitReason)
        {
            //TODO: Improve exception data;
            HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Unexpected HyperV exit_reason = {exitReason}", Sandbox.CorrelationId.Value!, MethodBase.GetCurrentMethod()!.DeclaringType!.Name);
        }

        internal override void Initialise(IntPtr pebAddress, ulong seed)
        {
            AddRegister(LinuxHyperV.HV_X64_REGISTER_RCX, (ulong)pebAddress, 0);
            AddRegister(LinuxHyperV.HV_X64_REGISTER_RDX, seed, 0);
            registers = registerList.ToArray();

            var setRegister = LinuxHyperV.set_registers(context.ctx, vcpuHandle.handle, registers, (UIntPtr)registers.Length);
            using (var registerHandle = new Handle(context, setRegister))
            {
                if (registerHandle.IsError())
                {
                    HyperlightException.LogAndThrowException<HyperVOnLinuxException>($"Failed setting registers Error: {registerHandle.GetErrorMessage()}", Sandbox.CorrelationId.Value!, GetType().Name);
                }
            }

            ExecuteUntilHalt();
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    if (Handle.Err((IntPtr)LinuxHyperV.unmap_vm_memory_region(context.ctx, vmHandle.handle, memoryHandle.handle)))
                    {
                        HyperlightLogger.LogError($"Error unmap VM memmory region. Error: {memoryHandle.GetErrorMessage()}", Sandbox.CorrelationId.Value!, GetType().Name);
                    }
                    memoryHandle.Dispose();
                    vcpuHandle.Dispose();
                    vmHandle.Dispose();
                    mshvHandle.Dispose();
                    context.Dispose();
                }

                disposedValue = true;
            }
        }

        ~HyperVOnLinux()
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
