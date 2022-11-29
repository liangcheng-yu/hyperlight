global using NativeHandle = System.UInt64;
global using NativeContext = System.IntPtr;
using System;
using Xunit;
using Hyperlight.Core;
using Hyperlight.Wrapper;
using System.Runtime.InteropServices;

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
        public void Test_Handle_Error()
        {
            var errStr = "TEST_HANDLE_ERROR";
            using var ctx = new Wrapper.Context();
            using var errHdl = Wrapper.Handle.NewError(ctx, errStr);
            Assert.True(errHdl.IsError());
            var retErrStr = errHdl.GetErrorMessage();
            Assert.Equal(errStr, retErrStr);
        }

        [Fact]
        public void Test_NewError_Empty_String()
        {
            using var ctx = new Wrapper.Context();
            using var errHdl = Wrapper.Handle.NewError(ctx, "");
            // errHdl should be an invalid handle rather than a valid error
            // because empty error strings should return invalid
            Assert.False(errHdl.IsError());
            Assert.False(errHdl.IsEmpty());
            Assert.True(errHdl.IsInvalid());
        }

        [Fact]
        public void Test_NewInvalidNullContext_Handle()
        {
            using var ctx = new Wrapper.Context();
            // Force a null context handle to be created so that we can validate that we are detecting the null context handle returned by the C API
            var rawHdl = int_32_new(NativeContext.Zero, 0);
            var hdl = new Handle(ctx, rawHdl);

            // Null context errors should be returned as a well-known handle
            Assert.False(hdl.IsError());
            Assert.False(hdl.IsEmpty());
            Assert.True(hdl.IsInvalid()); // NullContext should also be detected as an invalid handle
        }

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle int_32_new(
                    NativeContext ctx,
                    int val
                );

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
