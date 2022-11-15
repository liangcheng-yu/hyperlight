using System;
using System.Drawing;
using System.IO;
using System.Net;
using Hyperlight.Core;
using Hyperlight.Wrapper;
using Xunit;


namespace Hyperlight.Tests
{

    public class GuestMemoryTest
    {
        public GuestMemoryTest()
        {
            Assert.True(Sandbox.IsSupportedPlatform, "Hyperlight Sandbox is not supported on this platform.");

            // sandbox is only needed to initialise the context and correlation id.
            var options = SandboxHostTest.GetSandboxRunOptions();
            var path = AppDomain.CurrentDomain.BaseDirectory;
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = Path.Combine(path, guestBinaryFileName);
            using var _ = new Sandbox(guestBinaryPath, options[0]);
        }

        const ulong Size = 0x1000;

        [FactSkipIfNotWindowsAndNoHypervisor]
        public void Test_Copy_Array()
        {
            var val = Guid.NewGuid().ToByteArray();
            var offset = new IntPtr(0x100);
            using var mem = new GuestMemory(Size);
            mem.CopyFromByteArray(val, offset);
            var result = new byte[16];
            mem.CopyToByteArray(result, (ulong)offset);
            Assert.Equal(val, result);
        }
    }
}
