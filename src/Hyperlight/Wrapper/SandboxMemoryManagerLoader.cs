using System.Reflection.Metadata;
using System.Runtime.InteropServices;
using Hyperlight.Core;
namespace Hyperlight.Wrapper;

internal static class SandboxMemoryManagerLoader
{
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
