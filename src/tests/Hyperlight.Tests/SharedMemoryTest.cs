using System;
using System.IO;
using System.Linq;
using Hyperlight.Wrapper;
using Xunit;


namespace Hyperlight.Tests
{
    public class SharedMemoryTest
    {
        const ulong Size = 0x1000;

        [Fact]
        public void Test_Copy_Array()
        {
            using var ctx = new Context("sample_corr_id");
            var val = Guid.NewGuid().ToByteArray();
            var offset = new IntPtr(0x100);
            using (var mem = new SharedMemory(ctx, Size))
            {
                mem.CopyFromByteArray(val, offset);
                var result = new byte[16];
                mem.CopyToByteArray(result, (ulong)offset);
                Assert.Equal(val, result);
                Assert.Equal(
                    val,
                    mem.CopyAllToByteArray().Skip((int)offset).Take(result.Length)
                );
            }
        }

    }
}
