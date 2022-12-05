using System;
using Xunit;
using Hyperlight.Wrapper;
using System.Threading.Tasks;
using System.Threading;
using System.Collections.Generic;

namespace Hyperlight.Tests
{
    public class OutbHandlerWrapperTests
    {
        [Fact]
        public void Test_Constructor_Call()
        {
            using (var ctx = new Context())
            {
                bool called = false;
                ushort? calledPort = null;
                byte? calledPayload = null;

                Action<ushort, byte> action = (port, payload) =>
                {
                    called = true;
                    calledPort = port;
                    calledPayload = payload;
                };
                using (var wrapper = new OutbHandler(ctx, action))
                {
                    wrapper.Call(1, (byte)'a');
                    Assert.True(called);
                    Assert.Equal((ushort)1, calledPort);
                    Assert.Equal((byte)'a', calledPayload);
                }
            }
        }

        [Fact]
        public void Test_Constructor_Call_Concurrent()
        {
            const int numTasks = 10;
            using (var ctx = new Context())
            {
                int called = 0;

                Action<ushort, byte> action = (port, payload) =>
                {
                    Interlocked.Add(ref called, 1);
                };
                var handlers = new List<OutbHandler>();
                var handlersMut = new Mutex();
                for (int i = 0; i < numTasks; i++)
                {
                    // note, we need to dispose these OutbHandlers
                    //
                    // they have a handle into ctx, but also a 
                    // pointer to C#-unmanaged memory
                    handlers.Add(new OutbHandler(ctx, action));
                }

                Parallel.For(0, numTasks, (i, _) =>
                {
                    handlersMut.WaitOne();
                    var handler = handlers[i];
                    handlersMut.ReleaseMutex();
                    handler.Call(1, (byte)'a');
                });
                Assert.Equal(numTasks, called);

                handlers.ForEach((h) => h.Dispose());
            }
        }
    }
}
