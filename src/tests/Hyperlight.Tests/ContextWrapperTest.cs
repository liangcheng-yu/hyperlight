using System;
using Xunit;

namespace Hyperlight.Tests
{
    public class ContextWrapperTest
    {
        [Fact]
        public void Test_Create_Context()
        {
            using var ctx = new Wrapper.Context();
            var rawCtx = ctx.ctx;
            Assert.NotEqual(IntPtr.Zero, rawCtx);
        }
    }
}
