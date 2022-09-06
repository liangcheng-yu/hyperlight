using System;
using Xunit;
using Hyperlight.Core;
using Hyperlight.Wrapper;

namespace Hyperlight.Tests
{
    public class HandleWrapperTest
    {
        [Fact]
        public void Test_Ctor_Throw_Zero_Hdl()
        {
            using var ctx = new Wrapper.Context();
            Assert.Throws<HyperlightException>(
                () => new Wrapper.Handle(ctx, Wrapper.Handle.Zero)
            );
        }

        [Fact]
        public void Test_Raw_Hdl_Getter()
        {
            ulong rawHdl = 12345;
            using var ctx = new Wrapper.Context();
            using var hdl = new Wrapper.Handle(ctx, rawHdl);
            Assert.Equal(rawHdl, hdl.handle);
        }

        [Fact]
        public void Test_Dispose()
        {
            ulong rawHdl = 23456;
            using var ctx = new Wrapper.Context();
            var hdl = new Wrapper.Handle(ctx, rawHdl);
            Assert.NotEqual(Wrapper.Handle.Zero, hdl.handle);
            hdl.Dispose();
            Assert.Equal(Wrapper.Handle.Zero, hdl.handle);
        }

        [Fact]
        public void Test_NewError_ThrowIfError()
        {
            const String errMsg = "TEST ERR MSG";
            using var ctx = new Wrapper.Context();
            using var errHdl = Wrapper.Handle.NewError(ctx, errMsg);
            Assert.Throws<HyperlightException>(
                () => errHdl.ThrowIfError()
            );
        }

        [Fact]
        public void Test_Int_32()
        {
            const int val = 23456;
            using var ctx = new Wrapper.Context();
            using var hdl = Wrapper.Handle.NewInt32(ctx, val);
            Assert.False(hdl.IsInt64());
            Assert.True(hdl.IsInt32());
            Assert.Equal(val, hdl.GetInt32());
        }

        [Fact]
        public void Test_Int_64()
        {
            const long val = 12345;
            using var ctx = new Wrapper.Context();
            using var hdl = Wrapper.Handle.NewInt64(ctx, val);
            Assert.False(hdl.IsInt32());
            Assert.True(hdl.IsInt64());
            Assert.Equal(val, hdl.GetInt64());
        }
    }
}
