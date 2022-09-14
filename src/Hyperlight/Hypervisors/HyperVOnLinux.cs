using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Runtime.InteropServices;
using Hyperlight.Core;
using Hyperlight.Native;

namespace Hyperlight.Hypervisors
{
    internal class HyperVOnLinux : Hypervisor, IDisposable
    {
        private bool disposedValue;
        readonly IntPtr context = IntPtr.Zero;
        readonly IntPtr mshv_handle = IntPtr.Zero;
        readonly IntPtr vcpufd_handle = IntPtr.Zero;
        readonly IntPtr vmfd_handle = IntPtr.Zero;
        readonly IntPtr user_memory_region_handle = IntPtr.Zero;
        readonly List<LinuxHyperV.MSHV_REGISTER> registerList = new();
        LinuxHyperV.MSHV_REGISTER[]? registers;
        readonly LinuxHyperV.MSHV_REGISTER[] ripRegister = new LinuxHyperV.MSHV_REGISTER[1] { new() { Name = LinuxHyperV.HV_X64_REGISTER_RIP, Value = new LinuxHyperV.MSHV_U128 { HighPart = 0, LowPart = 0 } } };

        internal HyperVOnLinux(IntPtr sourceAddress, int pml4_addr, ulong size, ulong entryPoint, ulong rsp, Action<ushort, byte> outb, Action handleMemoryAccess) : base(sourceAddress, entryPoint, rsp, outb, handleMemoryAccess)

        {

            if (!LinuxHyperV.IsHypervisorPresent())
            {
                throw new HyperVOnLinuxException("HyperV Not Present");
            }

            if ((context = LinuxHyperV.context_new()) == IntPtr.Zero)
            {
                throw new HyperVOnLinuxException("Gettting context failed");
            }

            if (LinuxHyperV.handle_is_error(mshv_handle = LinuxHyperV.open_mshv(context, LinuxHyperV.REQUIRE_STABLE_API)))
            {
                throw new HyperVOnLinuxException("Unable to open mshv");
            }

            if (LinuxHyperV.handle_is_error(vmfd_handle = LinuxHyperV.create_vm(context, mshv_handle)))
            {
                throw new HyperVOnLinuxException("Unable to create HyperV VM");
            }

            if (LinuxHyperV.handle_is_error(vcpufd_handle = LinuxHyperV.create_vcpu(context, vmfd_handle)))
            {
                throw new HyperVOnLinuxException("Unable to create HyperV VCPU");
            }

            if (LinuxHyperV.handle_is_error(user_memory_region_handle = LinuxHyperV.map_vm_memory_region(context, vmfd_handle, (ulong)SandboxMemoryLayout.BaseAddress >> 12, (ulong)sourceAddress, size)))
            {
                throw new HyperVOnLinuxException("Failed to map User Memory Region");
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
            if (LinuxHyperV.handle_is_error(LinuxHyperV.set_registers(context, vcpufd_handle, ripRegister, (UIntPtr)ripRegister.Length)))
            {
                throw new HyperVOnLinuxException("Failed setting RIP");
            }
            ExecuteUntilHalt();
        }

        internal override void ExecuteUntilHalt()
        {
            var runResultHandle = IntPtr.Zero;
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
                    if (runResultHandle != IntPtr.Zero)
                    {
                        LinuxHyperV.handle_free(context, runResultHandle);
                        runResultHandle = IntPtr.Zero;
                    }
                    runResultHandle = LinuxHyperV.run_vcpu(context, vcpufd_handle);
                    if (LinuxHyperV.handle_is_error(runResultHandle))
                    {
                        throw new HyperVOnLinuxException("Failed to run VCPU");
                    }
                    runResultPtr = LinuxHyperV.get_run_result_from_handle(context, runResultHandle);
                    if (runResultPtr == IntPtr.Zero)
                    {
                        throw new HyperVOnLinuxException("Failed to get run result");
                    }
                    runResult = Marshal.PtrToStructure<LinuxHyperV.MSHV_RUN_MESSAGE>(runResultPtr);
                    switch (runResult.MessageType)
                    {
                        case LinuxHyperV.HV_MESSAGE_TYPE_HVMSG_X64_HALT:
                            break;
                        case LinuxHyperV.HV_MESSAGE_TYPE_HVMSG_X64_IO_PORT_INTERCEPT:
                            HandleOutb(runResult.PortNumber, Convert.ToByte(runResult.RAX));
                            ripRegister[0].Value.LowPart = runResult.RIP + runResult.InstructionLength;
                            if (LinuxHyperV.handle_is_error(LinuxHyperV.set_registers(context, vcpufd_handle, ripRegister, (UIntPtr)ripRegister.Length)))
                            {
                                throw new HyperVOnLinuxException("Failed setting RIP");
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
                while (runResult.MessageType != LinuxHyperV.HV_MESSAGE_TYPE_HVMSG_X64_HALT);
            }
            finally
            {

                if (runResultPtr != IntPtr.Zero)
                {
                    LinuxHyperV.free_run_result(runResultPtr);
                }
                if (runResultHandle != IntPtr.Zero)
                {
                    LinuxHyperV.handle_free(context, runResultHandle);
                }
            }
        }

        private static void ThrowExitException(uint exitReason)
        {
            //TODO: Improve exception data;
            throw new HyperVOnLinuxException($"Unexpected HyperV exit_reason = {exitReason}");
        }

        internal override void Initialise(IntPtr pebAddress, ulong seed)
        {
            AddRegister(LinuxHyperV.HV_X64_REGISTER_RCX, (ulong)pebAddress, 0);
            AddRegister(LinuxHyperV.HV_X64_REGISTER_RDX, seed, 0);
            registers = registerList.ToArray();

            if (LinuxHyperV.handle_is_error(LinuxHyperV.set_registers(context, vcpufd_handle, registers, (UIntPtr)registers.Length)))
            {
                throw new HyperVOnLinuxException("Failed setting registers");
            }
            ExecuteUntilHalt();
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    // TODO: dispose managed state (managed objects)
                }

                if (user_memory_region_handle != IntPtr.Zero)
                {
                    if (LinuxHyperV.handle_is_error(LinuxHyperV.unmap_vm_memory_region(context, vmfd_handle, user_memory_region_handle)))
                    {
                        // TODO: log the error
                    }

                    LinuxHyperV.handle_free(context, user_memory_region_handle);
                }

                if (vcpufd_handle != IntPtr.Zero)
                {
                    LinuxHyperV.handle_free(context, vcpufd_handle);
                }

                if (vmfd_handle != IntPtr.Zero)
                {
                    LinuxHyperV.handle_free(context, vmfd_handle);
                }

                if (mshv_handle != IntPtr.Zero)
                {
                    LinuxHyperV.handle_free(context, mshv_handle);
                }

                if (context != IntPtr.Zero)
                {
                    LinuxHyperV.context_free(context);
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
