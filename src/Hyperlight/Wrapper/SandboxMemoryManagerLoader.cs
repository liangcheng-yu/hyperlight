using System.Reflection.Metadata;
using System.Runtime.InteropServices;
using Hyperlight.Core;
namespace Hyperlight.Wrapper;

internal static class SandboxMemoryManagerLoader
{
    /// <summary>
    /// Load the guest binary at guestBinaryPath on disk into memory
    /// </summary>
    /// <param name="ctx">
    /// The Rust context to use to execute the load operation
    /// </param>
    /// <param name="memCfg">
    /// The memory configuration with which to configure guest memory
    /// </param>
    /// <param name="guestBinaryPath">
    /// The path on disk to the guest binary
    /// </param>
    /// <param name="runFromProcessMemory">
    /// Whether or not to run in-process
    /// </param>
    /// <param name="stackSizeOverride">
    /// The value to override the stack size specified in the guest binary's
    /// PE file header. Pass 0 to use the PE file value
    /// </param>
    /// <param name="heapSizeOverride">
    /// The value to override the heap size specified in the guest binary's
    /// PE file header. Pass 0 to use the PE file value
    /// </param>
    /// <returns>
    /// A new SandboxMemoryManager containing the loaded binary
    /// </returns>
    internal static Core.SandboxMemoryManager LoadGuestBinaryIntoMemory(
        Context ctx,
        SandboxMemoryConfiguration memCfg,
        string guestBinaryPath,
        bool runFromProcessMemory
    )
    {
        using var peInfo = new PEInfo(ctx, guestBinaryPath);
        var rawHdl = mem_mgr_load_guest_binary_into_memory(
            ctx.ctx,
            memCfg,
            peInfo.handleWrapper.handle,
            runFromProcessMemory
        );
        return Core.SandboxMemoryManager.FromHandle(
            ctx,
            new Handle(ctx, rawHdl, true)
        );
    }

    /// <summary>
    /// Load the given guest binary into memory using the windows LoadLibraryA
    /// call. 
    /// 
    /// WARNING: Calling this method crashes the process on Linux
    /// </summary>
    /// <param name="ctx">
    /// The Rust context to use to execute the load operation
    /// </param>
    /// <param name="memCfg">
    /// The memory configuration with which to configure guest memory
    /// </param>
    /// <param name="guestBinaryPath">
    /// The path on disk to the guest binary
    /// </param>
    /// <param name="runFromProcessMemory">
    /// Whether or not to run in-process
    /// </param>
    /// <param name="stackSizeOverride">
    /// The value to override the stack size specified in the guest binary's
    /// PE file header. Pass 0 to use the PE file value
    /// </param>
    /// <param name="heapSizeOverride">
    /// The value to override the heap size specified in the guest binary's
    /// PE file header. Pass 0 to use the PE file value
    /// </param>
    /// <returns>
    /// A new SandboxMemoryManager containing the loaded binary
    /// </returns>
    internal static Core.SandboxMemoryManager LoadGuestBinaryUsingLoadLibrary(
        Context ctx,
        SandboxMemoryConfiguration memCfg,
        string guestBinaryPath,
        bool runFromProcessMemory
    )
    {
        using var peInfo = new PEInfo(ctx, guestBinaryPath);
        using var guestBinPathHdl = StringWrapper.FromString(ctx, guestBinaryPath);
        var rawHdl = mem_mgr_load_guest_binary_using_load_library(
            ctx.ctx,
            memCfg,
            guestBinPathHdl.HandleWrapper.handle,
            peInfo.handleWrapper.handle,
            runFromProcessMemory
        );
        return Core.SandboxMemoryManager.FromHandle(
            ctx,
            new Handle(ctx, rawHdl, true)
        );

    }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory


    [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
    [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
    private static extern NativeHandle mem_mgr_load_guest_binary_into_memory(
        NativeContext ctx,
        SandboxMemoryConfiguration memCfg,
        NativeHandle peInfoHdl,
        [MarshalAs(UnmanagedType.U1)] bool runFromProcessMemory
    );

    [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
    [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
    private static extern NativeHandle mem_mgr_load_guest_binary_using_load_library(
        NativeContext ctx,
        SandboxMemoryConfiguration memCfg,
        NativeHandle guestBinNameHdl,
        NativeHandle peInfoHdl,
        [MarshalAs(UnmanagedType.U1)] bool runFromProcessMemory
    );
#pragma warning restore CA1707 // Remove the underscores from member name
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
}
