using System;
using System.Collections.Immutable;
using System.Reflection;
using System.Runtime.InteropServices;
using Hyperlight.Core;

namespace Hyperlight.Wrapper
{
    internal sealed class HostFunctionCallResultWrapper : IDisposable
    {
        private readonly Handle hdl;
        public Handle HandleWrapper
        {
            get
            {
                return hdl;
            }
        }
        private bool disposed;

        private HostFunctionCallResultWrapper(
            Context ctxWrapper,
            NativeHandle rawHdl
        )
        {
            this.hdl = new Handle(ctxWrapper, rawHdl, true);
        }


        public static HostFunctionCallResultWrapper FromObject(
            Context ctx,
            Type type,
            object? obj
        )
        {
            // TODO: support string, bool, void, byte[]
            var converters = ImmutableDictionary.Create<Type, Func<object?, HostFunctionCallResultWrapper>>()
            .Add(typeof(int), (obj) => From<int>(
                ctx,
                function_call_result_new_i32,
                ConvertTo<int>(obj)
            ))
            .Add(typeof(uint), (obj) => From<int>(
                ctx,
                function_call_result_new_i32,
                ConvertTo<int>(obj)
            ))
            .Add(typeof(long), (obj) => From<long>(
                ctx,
                function_call_result_new_i64,
                 ConvertTo<long>(obj)
            ))
            .Add(typeof(IntPtr), (obj) => From<long>(
                ctx,
                function_call_result_new_i64,
                ConvertTo<long>(obj)
            ))
            .Add(typeof(void), (obj) => FromVoid(ctx));

            if (converters.TryGetValue(type, out var converter))
            {
                return converter(obj);
            }

            var exceptionStr = $"Unsupported Host Method Return Type {type.FullName}";
            HyperlightException.LogAndThrowException<ArgumentException>(
                exceptionStr,
                MethodBase.GetCurrentMethod()!.DeclaringType!.Name
            );

            // this throw is not necessary at runtime, as the above 
            // LogAndThrowException call will throw. we include it
            // to satisfy the compiler that all code paths either return
            // a value or throw
            throw new HyperlightException(exceptionStr);

        }
        private static HostFunctionCallResultWrapper From<T>(
            Context ctx,
            Func<NativeContext, T, NativeHandle> applyFn,
            T val
        )
        {
            var rawHdl = applyFn(ctx.ctx, val);
            return new HostFunctionCallResultWrapper(ctx, rawHdl);
        }

        private static T ConvertTo<T>(object? obj)
        {
            if (obj is null)
            {
                var errStr = $"Trying to convert null to type {typeof(T)}";
                HyperlightException.LogAndThrowException<ArgumentException>(
                    errStr,
                    MethodBase.GetCurrentMethod()!.DeclaringType!.Name
                );
                throw new ArgumentException(errStr);
            }

            // This is a hack need a better way to do conversions but should do this when adding support for all host return types
            // This is needed as an InPtr cannot be cast to a long

            if (obj is IntPtr ptr)
            {
                obj = ptr.ToInt64();
            }

            return (T)obj;
        }

        private static HostFunctionCallResultWrapper FromVoid(Context ctx)
        {
            return new HostFunctionCallResultWrapper(
                ctx,
                function_call_result_new_void(ctx.ctx)
            );
        }

        public void Dispose()
        {
            this.Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }

        private void Dispose(bool disposing)
        {
            if (!this.disposed)
            {
                if (disposing)
                {
                    this.hdl.Dispose();
                }
                this.disposed = true;
            }
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern unsafe NativeHandle function_call_result_new_i32(
            NativeContext ctx,
            int val
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern unsafe NativeHandle function_call_result_new_i64(
            NativeContext ctx,
            long val
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern unsafe NativeHandle function_call_result_new_bool(
            NativeContext ctx,
            bool val
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern unsafe NativeHandle function_call_result_new_void(
            NativeContext ctx
        );

#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
#pragma warning restore CA1707


    }
}
