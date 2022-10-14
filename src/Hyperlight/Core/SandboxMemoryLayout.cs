using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;

namespace Hyperlight.Core
{
    // The folllowing structs are not used other than to calculate the size of the memory needed 
    // and also to illustrate the layout of the memory

    // the start of the guest  memory contains the page tables and is always located at the Virtual Address 0x200000 when running in a Hypervisor:

    // Virtual Address
    //
    // 0x200000    PML4
    // 0x201000    PDPT
    // 0x202000    PD
    // 0x203000    The guest PE code (When the code has been loaded using LoadLibrary to debug the guest this will not be present and code length will be zero;

    // The pointer passed to the Entrypoint in the Guest application is  0x200000 + size of page table + size of code, at this address structs below are laid out in this order

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct GuestSecurityCookie
    {
        internal ulong SecurityCookieSeed;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct HostFunctionDefinitions
    {
        internal ulong FunctionDefinitionSize;
        internal IntPtr FunctionDefinitions;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct HostExceptionData
    {
        internal ulong HostExceptionSize;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct GuestError
    {
        internal GuestErrorCode ErrorCode;
        internal ulong MaxMessageSize;
        internal IntPtr Message;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct CodeAndOutBPointers
    {
        internal IntPtr CodePointer;
        internal IntPtr OutBPointer;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct InputData
    {
        internal ulong InputDataSize;
        internal IntPtr InputDataBuffer;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct OutputData
    {
        internal ulong OutputDataSize;
        internal IntPtr OutputDataBuffer;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct GuestHeap
    {
        internal ulong GuestHeapSize;
        internal IntPtr GuestHeapBuffer;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    struct GuestStack
    {
        internal IntPtr MinGuestStackAddress;
    }

    // Following these structures are the memory buffers as follows:

    // Host Function definitions - length sandboxMemoryConfiguration.HostFunctionDefinitionSize

    // Host Exception Details - length sandboxMemoryConfiguration.HostExceptionSize , this contains details of any Host Exception that occurred in outb function 
    // it contains a 32 bit length following by a json serialisation of any error that occurred.

    // Guest Error Details - length sandboxMemoryConfiguration.GuestErrorMessageSize this contains an error message string for any guest error that occurred.

    // Input Data Buffer - length sandboxMemoryConfiguration.InputDataSize this is a buffer that is used for input data to the host program

    // Output Data Buffer - length sandboxMemoryConfiguration.OutputDataSize this is a buffer that is used for output data from host program

    // Guest Heap - length heapSize this is a memory buffer provided to the guest to be used as heap memory.

    // Guest Stack - length stackSize this is the memory used for the guest stack (in reality the stack may be slightly bigger or smaller as the total memory size is rounded up to nearest 4K and there is a 16 bte stack guard written to the top of the stack).

    internal class SandboxMemoryLayout
    {

        const int PageTableSize = 0x3000;
        const int PDOffset = 0x2000;
        const int PDPTOffset = 0x1000;
        const int CodeOffSet = PageTableSize;
        const long maxMemorySize = 0x3FEF0000;

        public const int BaseAddress = 0x200000;
        public const int PML4GuestAddress = BaseAddress;
        public const ulong PDPTGuestAddress = BaseAddress + PDPTOffset;
        public const int PDGuestAddress = BaseAddress + PDOffset;
        public const ulong GuestCodeAddress = BaseAddress + CodeOffSet;

        readonly int pebOffset;
        readonly long stackSize;
        readonly long heapSize;
        readonly int hostFunctionsOffset;
        readonly int hostExceptionOffset;
        readonly int guestErrorMessageOffset;
        readonly int codeandOutbPointerOffset;
        readonly int inputDataOffset;
        readonly int outputDataOffset;
        readonly int heapDataOffset;
        readonly int stackDataOffset;
        readonly SandboxMemoryConfiguration sandboxMemoryConfiguration;
        readonly int codeSize;
        readonly int hostFunctionsBufferOffset;
        readonly int hostExceptionBufferOffset;
        readonly int guestSecurityCookieSeedOffset;
        readonly int guestErrorMessageBufferOffset;
        readonly int inputDataBufferOffset;
        readonly int outputDataBufferOffset;
        readonly int guestHeapBufferOffset;
        readonly long guestStackBufferOffset;

        public ulong PEBAddress { get; init; }

        IntPtr GetSecurityCookieSeedAddress(IntPtr address) => IntPtr.Add(address, guestSecurityCookieSeedOffset);
        IntPtr GetGuestErrorMessageAddress(IntPtr address) => IntPtr.Add(address, guestErrorMessageBufferOffset);
        public IntPtr GetGuestErrorAddress(IntPtr address) => IntPtr.Add(address, guestErrorMessageOffset);
        IntPtr GetGuestErrorMessageSizeAddress(IntPtr address) => IntPtr.Add(GetGuestErrorAddress(address), sizeof(ulong)); // Size of error message is after the GuestErrorMessage field which is a ulong.
        public IntPtr GetGuestErrorMessagePointerAddress(IntPtr address) => IntPtr.Add(GetGuestErrorMessageSizeAddress(address), sizeof(ulong)); // Pointer to the error message is after the Size field which is a ulong.
        public IntPtr GetFunctionDefinitionAddress(IntPtr address) => IntPtr.Add(address, hostFunctionsBufferOffset);
        IntPtr GetFunctionDefinitionSizeAddress(IntPtr address) => IntPtr.Add(address, hostFunctionsOffset);
        IntPtr GetFunctionDefinitionPointerAddress(IntPtr address) => IntPtr.Add(GetFunctionDefinitionSizeAddress(address), sizeof(ulong)); // Pointer to functions data is after the size field which is a ulong.
        IntPtr GetHostExceptionSizeAddress(IntPtr address) => IntPtr.Add(address, hostExceptionOffset);
        public IntPtr GetHostExceptionAddress(IntPtr address) => IntPtr.Add(address, hostExceptionBufferOffset);
        public IntPtr GetOutBPointerAddress(IntPtr address) => IntPtr.Add(GetCodePointerAddress(address), sizeof(ulong)); // OutB pointer is after the Code Pointer field which is a ulong.. 
        IntPtr GetOutputDataSizeAddress(IntPtr address) => IntPtr.Add(address, outputDataOffset);
        IntPtr GetOutputDataPointerAddress(IntPtr address) => IntPtr.Add(GetOutputDataSizeAddress(address), sizeof(ulong)); // Pointer to input data is after the size field which is a ulong.
        public IntPtr GetOutputDataAddress(IntPtr address) => IntPtr.Add(address, outputDataBufferOffset);
        IntPtr GetInputDataSizeAddress(IntPtr address) => IntPtr.Add(address, inputDataOffset);
        IntPtr GetInputDataPointerAddress(IntPtr address) => IntPtr.Add(GetInputDataSizeAddress(address), sizeof(ulong)); // Pointer to input data is after the size field which is a ulong.
        public IntPtr GetInputDataAddress(IntPtr address) => IntPtr.Add(address, inputDataBufferOffset);
        public IntPtr GetCodePointerAddress(IntPtr address) => IntPtr.Add(address, codeandOutbPointerOffset);
        public IntPtr GetDispatchFunctionPointerAddress(IntPtr address) => IntPtr.Add(GetFunctionDefinitionAddress(address), sizeof(ulong)); // Pointer to Dispatch Function is offset eight bytes into the FunctionDefinition.
        public ulong GetInProcessPEBAddress(IntPtr address) => (ulong)(address + pebOffset);
        IntPtr GetHeapSizeAddress(IntPtr address) => IntPtr.Add(address, heapDataOffset);
        IntPtr GetHeapPointerAddress(IntPtr address) => IntPtr.Add(GetHeapSizeAddress(address), sizeof(ulong)); // Pointer to heap data is after the size field which is a ulong.
        IntPtr GetHeapAddress(IntPtr address) => IntPtr.Add(address, guestHeapBufferOffset);
        IntPtr GetMinGuestStackAddressPointer(IntPtr address) => IntPtr.Add(address, stackDataOffset);
        public IntPtr GetTopOfStackAddress(IntPtr address) => (IntPtr)((long)address + guestStackBufferOffset);

        public static IntPtr GetHostPML4Address(IntPtr address) => address;
        public static IntPtr GetHostPDPTAddress(IntPtr address) => IntPtr.Add(address, PDPTOffset);
        public static IntPtr GetHostPDAddress(IntPtr address) => IntPtr.Add(address, PDOffset);
        public static IntPtr GetHostCodeAddress(IntPtr address) => IntPtr.Add(address, CodeOffSet);

        internal SandboxMemoryLayout(SandboxMemoryConfiguration sandboxMemoryConfiguration, int codeSize, long stackSize, long heapSize)
        {
            this.sandboxMemoryConfiguration = sandboxMemoryConfiguration;
            this.codeSize = codeSize;
            this.stackSize = stackSize;
            this.heapSize = heapSize;
            pebOffset = PageTableSize + codeSize;
            guestSecurityCookieSeedOffset = PageTableSize + codeSize;
            hostFunctionsOffset = guestSecurityCookieSeedOffset + Marshal.SizeOf(typeof(GuestSecurityCookie));
            hostExceptionOffset = hostFunctionsOffset + Marshal.SizeOf(typeof(HostFunctionDefinitions));
            guestErrorMessageOffset = hostExceptionOffset + Marshal.SizeOf(typeof(HostExceptionData));
            codeandOutbPointerOffset = guestErrorMessageOffset + Marshal.SizeOf(typeof(GuestError));
            inputDataOffset = codeandOutbPointerOffset + Marshal.SizeOf(typeof(CodeAndOutBPointers));
            outputDataOffset = inputDataOffset + Marshal.SizeOf(typeof(InputData));
            heapDataOffset = outputDataOffset + Marshal.SizeOf(typeof(OutputData));
            stackDataOffset = heapDataOffset + Marshal.SizeOf(typeof(GuestHeap));
            PEBAddress = (ulong)(BaseAddress + pebOffset);
            hostFunctionsBufferOffset = stackDataOffset + Marshal.SizeOf(typeof(GuestStack));
            hostExceptionBufferOffset = hostFunctionsBufferOffset + (int)sandboxMemoryConfiguration.HostFunctionDefinitionSize;
            guestErrorMessageBufferOffset = hostExceptionBufferOffset + (int)sandboxMemoryConfiguration.HostExceptionSize;
            inputDataBufferOffset = guestErrorMessageBufferOffset + (int)sandboxMemoryConfiguration.GuestErrorMessageSize;
            outputDataBufferOffset = inputDataBufferOffset + (int)sandboxMemoryConfiguration.InputDataSize;
            guestHeapBufferOffset = outputDataBufferOffset + (int)sandboxMemoryConfiguration.OutputDataSize;
            guestStackBufferOffset = guestHeapBufferOffset + heapSize;
        }

        internal ulong GetMemorySize()
        {
            var totalMemory = codeSize
                              + PageTableSize
                              + (int)sandboxMemoryConfiguration.HostFunctionDefinitionSize
                              + (int)sandboxMemoryConfiguration.InputDataSize
                              + (int)sandboxMemoryConfiguration.OutputDataSize
                              + (int)sandboxMemoryConfiguration.HostExceptionSize
                              + (int)sandboxMemoryConfiguration.GuestErrorMessageSize
                              + Marshal.SizeOf(typeof(GuestSecurityCookie))
                              + Marshal.SizeOf(typeof(HostFunctionDefinitions))
                              + Marshal.SizeOf(typeof(HostExceptionData))
                              + Marshal.SizeOf(typeof(GuestError))
                              + Marshal.SizeOf(typeof(CodeAndOutBPointers))
                              + Marshal.SizeOf(typeof(InputData))
                              + Marshal.SizeOf(typeof(OutputData))
                              + Marshal.SizeOf(typeof(GuestHeap))
                              + Marshal.SizeOf(typeof(GuestStack))
                              + heapSize
                              + stackSize;

            // Size should be a multiple of 4K.

            var remainder = totalMemory % 0x1000;
            var multiples = totalMemory / 0x1000;
            var size = (ulong)(remainder > 0 ? (multiples + 1) * 0x1000 : totalMemory);

            // For our page table, we only mapped virtual memory up to 0x3FFFFFFF and map each 2 meg 
            // virtual chunk to physical addresses 2 megabytes below the virtual address.  Since we
            // map virtual up to 0x3FFFFFFF, the max physical address we handle is 0x3FDFFFFF (or 
            // 0x3FEF0000 physical total memory)

            if (size > maxMemorySize)
            {
                HyperlightException.LogAndThrowException<ArgumentException>($"Total memory size {size} exceeds limit of {maxMemorySize}", Sandbox.CorrelationId.Value!, GetType().Name);
            }

            return size;
        }
        internal void WriteMemoryLayout(IntPtr sourceAddress, IntPtr guestAddress, ulong size)
        {
            // Set up the security cookie seed
            var securityCookieSeed = new byte[8];
            using (var randomNumberGenerator = RandomNumberGenerator.Create())
            {
                randomNumberGenerator.GetBytes(securityCookieSeed);
            }
            var securityCookieSeedPointer = GetSecurityCookieSeedAddress(sourceAddress);
            Marshal.Copy(securityCookieSeed, 0, securityCookieSeedPointer, securityCookieSeed.Length);

            // Set up Guest Error Header
            var errorMessageSizePointer = GetGuestErrorMessageSizeAddress(sourceAddress);
            Marshal.WriteInt64(errorMessageSizePointer, (int)sandboxMemoryConfiguration.GuestErrorMessageSize);
            var errorMessagePointerAddress = GetGuestErrorMessagePointerAddress(sourceAddress);
            var errorMessageAddress = GetGuestErrorMessageAddress(guestAddress);
            Marshal.WriteIntPtr(errorMessagePointerAddress, errorMessageAddress);

            // Set up Host Exception Header
            var hostExceptionSizePointer = GetHostExceptionSizeAddress(sourceAddress);
            Marshal.WriteInt64(hostExceptionSizePointer, (int)sandboxMemoryConfiguration.HostExceptionSize);

            // Set up input buffer pointer

            var inputDataSizePointer = GetInputDataSizeAddress(sourceAddress);
            Marshal.WriteInt64(inputDataSizePointer, (int)sandboxMemoryConfiguration.InputDataSize);
            var inputDataPointerAddress = GetInputDataPointerAddress(sourceAddress);
            var inputDataAddress = GetInputDataAddress(guestAddress);
            Marshal.WriteIntPtr(inputDataPointerAddress, inputDataAddress);

            // Set up output buffer pointer

            var outputDataSizePointer = GetOutputDataSizeAddress(sourceAddress);
            Marshal.WriteInt64(outputDataSizePointer, (int)sandboxMemoryConfiguration.OutputDataSize);
            var outputDataPointerAddress = GetOutputDataPointerAddress(sourceAddress);
            var outputDataAddress = GetOutputDataAddress(guestAddress);
            Marshal.WriteIntPtr(outputDataPointerAddress, outputDataAddress);

            // Set up heap buffer pointer

            var heapSizePointer = GetHeapSizeAddress(sourceAddress);
            Marshal.WriteInt64(heapSizePointer, heapSize);
            var heapPointerAddress = GetHeapPointerAddress(sourceAddress);
            var heapAddress = GetHeapAddress(guestAddress);
            Marshal.WriteIntPtr(heapPointerAddress, heapAddress);

            // Set up Function Definition Header
            var functionDefinitionSizePointer = GetFunctionDefinitionSizeAddress(sourceAddress);
            Marshal.WriteInt64(functionDefinitionSizePointer, (int)sandboxMemoryConfiguration.HostFunctionDefinitionSize);
            var functionDefinitionPointerAddress = GetFunctionDefinitionPointerAddress(sourceAddress);
            var functionDefinitionAddress = GetFunctionDefinitionAddress(guestAddress);
            Marshal.WriteIntPtr(functionDefinitionPointerAddress, functionDefinitionAddress);

            // Set up Max Guest Stack Address
            var minGuestStackAddressPointer = GetMinGuestStackAddressPointer(sourceAddress);
            var minGuestStackAddressOffset = size - (ulong)stackSize;
            var minGuestStackAddress = (long)guestAddress + (long)minGuestStackAddressOffset;
            Marshal.WriteInt64(minGuestStackAddressPointer, minGuestStackAddress);
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_layout_new(
            NativeContext ctx,
            NativeHandle mem_config_handle,
            ulong code_size,
            ulong stack_size,
            ulong heap_size
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_stack_size(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_heap_size(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_host_functions_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_host_exception_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );


        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_guest_error_message_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_code_and_outb_pointer_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_input_data_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_output_data_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_heap_data_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_stack_data_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_code_size(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_host_functions_buffer_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_host_exception_buffer_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_guest_security_cookie_seed_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_guest_error_message_buffer_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_input_data_buffer_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_output_data_buffer_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_guest_heap_buffer_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_guest_stack_buffer_offset(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_peb_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_security_cookie_seed_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_guest_error_message_address
        (
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_guest_error_address
        (
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_guest_error_message_size_address(
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_guest_error_message_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_function_definition_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_function_definition_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_function_definition_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_host_exception_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_host_exception_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_out_b_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_output_data_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_output_data_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_output_data_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_input_data_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_input_data_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_input_data_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_code_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_dispatch_function_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_in_process_peb_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_heap_size_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_heap_pointer_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_heap_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_min_guest_stack_address_pointer(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_top_of_stack_address(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            long address
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_host_pml4_address(
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_host_pdpt_address(
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_host_pd_address(
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_host_code_address(
            long address
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern void mem_layout_write_memory_layout(
            NativeContext ctx,
            NativeHandle mem_layout_handle,
            NativeHandle guest_memory_handle,
            long address,
            long guestAddress,
            ulong size
        );
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern ulong mem_layout_get_memory_size(
            NativeContext ctx,
            NativeHandle mem_layout_handle
        );

#pragma warning restore CA5393
#pragma warning restore CA1707
    }
}
