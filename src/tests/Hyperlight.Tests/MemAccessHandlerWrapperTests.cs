using System;
using Xunit;
using Hyperlight.Wrapper;

namespace Hyperlight.Tests
{
    public class MemAccessHandlerTests
    {
        [Fact]
        public void Test_Constructor_Call()
        {
            using (var ctx = new Context())
            {
                bool called = false;
                Action action = () =>
                {
                    called = true;
                };
                using (var wrapper = new MemAccessHandler(ctx, action))
                {
                    wrapper.Call();
                    Assert.True(called);
                }
            }
        }
    }
}
