using System;
using System.Linq;
using System.IO;
using Hyperlight.Wrapper;
using Xunit;

namespace Hyperlight.Tests
{

    public class GuestMemorySnapshotTest
    {
        public GuestMemorySnapshotTest()
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
        public void Test_Create_Replace_Restore()
        {
            // not the most efficient way to initialize a long byte array, 
            // but I believe this is efficient enough and readable enough.
            // taken from https://stackoverflow.com/a/6150150
            byte[] data1 = Enumerable.Repeat((byte)0x3, (int)Size).ToArray();
            byte[] data2 = Enumerable.Repeat((byte)0x4, (int)Size).ToArray();
            using var gm = new GuestMemory(Size);
            gm.CopyFromByteArray(data1, new IntPtr(0));
            using var snap = new GuestMemorySnapshot(Sandbox.Context.Value!, gm);
            {
                // after the first snapshot is taken, make sure gm has the equivalent
                // of data1
                Assert.Equal(data1, gm.CopyAllToByteArray());
            }
            {
                // modify gm with data2 rather than data1 and restore from
                // snapshot. we should have the equivalent of data1 again
                gm.CopyFromByteArray(data2, new IntPtr(0));
                Assert.Equal(data2, gm.CopyAllToByteArray());
                snap.RestoreFromSnapshot();
                Assert.Equal(data1, gm.CopyAllToByteArray());
            }
            {
                // modify gm with data2, then retake the snapshot and restore
                // from the new snapshot. we should have the equivalent of data2
                gm.CopyFromByteArray(data2, new IntPtr(0));
                Assert.Equal(data2, gm.CopyAllToByteArray());
                snap.ReplaceSnapshot();
                Assert.Equal(data2, gm.CopyAllToByteArray());
                snap.RestoreFromSnapshot();
                Assert.Equal(data2, gm.CopyAllToByteArray());
            }
        }
    }
}
