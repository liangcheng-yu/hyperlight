using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using Google.FlatBuffers;
using Hyperlight.Core;
using Hyperlight.Generated;
using Newtonsoft.Json;

namespace Hyperlight.Wrapper
{
    internal sealed class SandboxMemoryManager : IDisposable
    {
        private readonly Context ctxWrapper;
        private readonly Handle memMgrHdl;

        /// <summary>
        /// Get the Handle backing this SandboxMemoryManager
        /// </summary>
        public Handle Handle => memMgrHdl;
        private bool disposedValue;

        /// <summary>
        /// Get the offset, from the start of memory (loadAddr), to the entrypoint
        /// </summary>
        private ulong entryPointOffset
        {
            get
            {
                var rawHdl = mem_mgr_get_entrypoint_offset(
                    this.ctxWrapper.ctx,
                    this.memMgrHdl.handle
                );
                using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
                if (!hdl.IsUInt64())
                {
                    throw new HyperlightException(
                        "mem_mgr_get_entrypoint_offset did not return a uint64"
                    );
                }
                return hdl.GetUInt64();
            }
        }

        /// <summary>
        /// Get the address of the start of memory on the host.
        /// </summary>
        private IntPtr loadAddr
        {
            get
            {
                var rawHdl = mem_mgr_get_load_addr(
                    this.ctxWrapper.ctx,
                    this.memMgrHdl.handle
                );
                using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
                if (!hdl.IsUInt64())
                {
                    throw new HyperlightException(
                        "mem_mgr_get_load_addr did not return a uint64"
                    );
                }
                return new IntPtr((long)hdl.GetUInt64());
            }
        }

        public ulong EntryPoint => (ulong)IntPtr.Add(
            this.loadAddr,
            (int)this.entryPointOffset
        ).ToInt64();

        private SharedMemory SharedMem
        {
            get
            {
                return new SharedMemory(
                    this.ctxWrapper,
                    ctx => mem_mgr_get_shared_memory(
                        ctx.ctx,
                        this.memMgrHdl.handle
                    )
                );
            }
        }
        public IntPtr SourceAddress => this.SharedMem.Address;

        public ulong Size
        {
            get
            {
                var rawHdl = mem_mgr_get_mem_size(
                    this.ctxWrapper.ctx,
                    this.memMgrHdl.handle
                );
                using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
                if (!hdl.IsUInt64())
                {
                    throw new HyperlightException(
                        "mem_mgr_get_mem_size did not return a uint64"
                    );
                }
                return hdl.GetUInt64();
            }
        }

        public ulong GuardPageOffset
        {
            get
            {
                var rawHdl = mem_mgr_get_guard_page_offset(
                    this.ctxWrapper.ctx,
                    this.memMgrHdl.handle
                );
                using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
                if (!hdl.IsUInt64())
                {
                    throw new HyperlightException(
                        "mem_mgr_get_guard_page_offset did not return a uint64"
                    );
                }
                return hdl.GetUInt64();
            }
        }

        internal SandboxMemoryManager(
            Context ctx,
            Handle hdl
        )
        {
            this.ctxWrapper = ctx;
            hdl.ThrowIfUnusable();
            this.memMgrHdl = hdl;
        }

        internal ulong SetUpHyperVisorPartition()
        {
            var rawHdl = mem_mgr_set_up_hypervisor_partition(
                this.ctxWrapper.ctx,
                this.memMgrHdl.handle,
                this.Size
            );
            using var hdl = new Handle(
                this.ctxWrapper,
                rawHdl,
                true
            );
            if (!hdl.IsUInt64())
            {
                throw new HyperlightException("mem_mgr_set_up_hypervisor_partition did not return a UInt64");
            }
            return hdl.GetUInt64();
        }

        internal ulong GetPebAddress()
        {
            var rawHdl = mem_mgr_get_peb_address(
                this.ctxWrapper.ctx,
                this.memMgrHdl.handle,
                (ulong)this.SourceAddress.ToInt64()
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsUInt64())
            {
                throw new HyperlightException("mem_mgr_get_peb_address did not return a uint64");
            }
            return hdl.GetUInt64();
        }

        internal void SnapshotState()
        {
            var rawHdl = mem_mgr_snapshot_state(
                this.ctxWrapper.ctx,
                this.memMgrHdl.handle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
        }

        internal void RestoreState()
        {
            var rawHdl = mem_mgr_restore_state(
                this.ctxWrapper.ctx,
                this.memMgrHdl.handle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
        }

        internal object GetReturnValue()
        {

            using var resultHdlWrapper = new Handle(
                this.ctxWrapper,
                mem_mgr_get_function_call_result(
                    this.ctxWrapper.ctx,
                    this.memMgrHdl.handle
                ),
                true
            );

            if (!resultHdlWrapper.IsFunctionCallResult())
            {
                throw new HyperlightException("mem_mgr_get_function_call_result did not return a FunctionCallResult");
            }
            var functionCallResult = resultHdlWrapper.GetFunctionCallResult();

            return functionCallResult.ReturnValueType switch
            {
                ReturnValue.hlint => functionCallResult.ReturnValueAshlint().Value,
                ReturnValue.hllong => functionCallResult.ReturnValueAshllong().Value,
                ReturnValue.hlstring => functionCallResult.ReturnValueAshlstring().Value,
                ReturnValue.hlbool => functionCallResult.ReturnValueAshlbool().Value,
                ReturnValue.hlvoid => functionCallResult.ReturnValueAshlvoid(),
                ReturnValue.hlsizeprefixedbuffer => functionCallResult.ReturnValueAshlsizeprefixedbuffer().GetValueArray(),
                _ => throw new HyperlightException($"ReturnValueType {functionCallResult.ReturnValueType} was not expected"),
            };
        }

        internal int GetInitReturnValue()
        {
            var rawHdl = mem_mgr_get_return_value(
                this.ctxWrapper.ctx,
                this.memMgrHdl.handle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsInt32())
            {
                throw new HyperlightException(
                    "handle returned from mem_mgr_get_return_value was not an int32"
                );
            }
            return hdl.GetInt32();
        }

        internal void SetOutBAddress(long pOutB)
        {
            var rawHdl = mem_mgr_set_outb_address(
                this.ctxWrapper.ctx,
                this.memMgrHdl.handle,
                (ulong)pOutB
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
        }

        internal ulong GetPointerToDispatchFunction()
        {
            var rawHdl = mem_mgr_get_pointer_to_dispatch_function(
                this.ctxWrapper.ctx,
                this.memMgrHdl.handle
            );
            using var hdl = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl.IsUInt64())
            {
                throw new HyperlightException(
                    "mem_mgr_get_pointer_to_dispatch_function did not return a uint64"
                );
            }
            return hdl.GetUInt64();
        }

        internal HyperlightException? GetHostException()
        {
            HyperlightException? hyperlightException = null;
            var rawHdl = mem_mgr_has_host_exception(
                this.ctxWrapper.ctx,
                this.memMgrHdl.handle
            );
            using var hdl1 = new Handle(this.ctxWrapper, rawHdl, true);
            if (!hdl1.IsBoolean())
            {
                throw new HyperlightException("mem_mgr_has_host_exception did not return a boolean");
            }
            var hasException = hdl1.GetBoolean();
            if (hasException)
            {
                rawHdl = mem_mgr_get_host_exception_length(
                    this.ctxWrapper.ctx,
                    this.memMgrHdl.handle
                );
                using var hdl2 = new Handle(this.ctxWrapper, rawHdl, true);
                if (!hdl2.IsInt32())
                {
                    throw new HyperlightException("mem_mgr_get_host_exception_length did not return an int32");
                }
                var size = hdl2.GetInt32();
                if (size == 0)
                {
                    throw new HyperlightException("mem_mgr_get_host_exception_length returned 0");
                }
                var data = new byte[size];
                unsafe
                {
                    fixed (byte* exceptionDataPtr = data)
                    {
                        using var resultHdlWrapper = new Handle(
                            this.ctxWrapper,
                            mem_mgr_get_host_exception_data(
                                this.ctxWrapper.ctx,
                                this.memMgrHdl.handle,
                                (IntPtr)exceptionDataPtr,
                                size
                            ),
                            true
                        );
                    }
                }
                var exceptionAsJson = Encoding.UTF8.GetString(data);
                // TODO: Switch to System.Text.Json - requires custom serialisation as default throws an exception when serialising if an inner exception is present
                // as it contains a Type: System.NotSupportedException: Serialization and deserialization of 'System.Type' instances are not supported and should be avoided since they can lead to security issues.
                // https://docs.microsoft.com/en-us/dotnet/standard/serialization/system-text-json-converters-how-to?pivots=dotnet-6-0
#pragma warning disable CA2326 // Do not use TypeNameHandling values other than None - this will be fixed by the above TODO
#pragma warning disable CA2327 // Do not use SerializationBinder classes - this will be fixed by the above TODO 
                hyperlightException = JsonConvert.DeserializeObject<HyperlightException>(exceptionAsJson, new JsonSerializerSettings
                {
                    TypeNameHandling = TypeNameHandling.Auto
                });
#pragma warning restore CA2326 // Do not use TypeNameHandling values other than None
#pragma warning restore CA2327 // Do not use SerializationBinder classes
            }
            return hyperlightException;
        }

        internal void WriteOutbException(Exception ex, ushort port)
        {
            var data = Encoding.UTF8.GetBytes($"Port:{port}, Message:{ex.Message}\0");
            using var errorMessage = new ByteArray(
                this.ctxWrapper,
                data
            );

            var hyperLightException = ex.GetType() == typeof(HyperlightException) ? ex as HyperlightException : new HyperlightException($"OutB Error {ex.Message}", ex);

            // TODO: Switch to System.Text.Json - requires custom serialisation as default throws an exception when serialising if an inner exception is present
            // as it contains a Type: System.NotSupportedException: Serialization and deserialization of 'System.Type' instances are not supported and should be avoided since they can lead to security issues.
            // https://docs.microsoft.com/en-us/dotnet/standard/serialization/system-text-json-converters-how-to?pivots=dotnet-6-0
#pragma warning disable CA2326 // Do not use TypeNameHandling values other than None - this will be fixed by the above TODO
            var exceptionAsJson = JsonConvert.SerializeObject(hyperLightException, new JsonSerializerSettings
            {
                TypeNameHandling = TypeNameHandling.Auto
            });

#pragma warning restore CA2326 // Do not use TypeNameHandling values other than None
            data = Encoding.UTF8.GetBytes(exceptionAsJson);
            using var exceptionData = new ByteArray(
                this.ctxWrapper,
                data
            );

            using var resultHdlWrapper = new Handle(
                this.ctxWrapper,
                mem_mgr_write_outb_exception(
                    this.ctxWrapper.ctx,
                    this.memMgrHdl.handle,
                    errorMessage.handleWrapper.handle,
                    exceptionData.handleWrapper.handle),
                true
            );

            HyperlightLogger.LogError($"Exception occurred in outb", GetType().Name, ex);
        }

        internal (ErrorCode ErrorCode, string? Message) GetGuestError()
        {

            using var resultHdlWrapper = new Handle(
                this.ctxWrapper,
                mem_mgr_get_guest_error(
                    this.ctxWrapper.ctx,
                    this.memMgrHdl.handle
                ),
                true
            );

            if (!resultHdlWrapper.IsGuestError())
            {
                throw new HyperlightException("mem_mgr_get_guest_error did not return a GuestError");
            }
            var guestError = resultHdlWrapper.GetGuestError();
            return (guestError.Code, guestError.Message);

        }

        internal void WriteGuestFunctionCallDetails(string functionName, object[] args, RuntimeTypeHandle returnType)
        {

            var builder = new FlatBufferBuilder(1024);
            var funcName = builder.CreateString(functionName);
            var nextArgShouldBeArrayLength = false;
            var nextArgLength = 0;
            var parameters = new Offset<Parameter>[args.Length];
            for (var i = 0; i < args.Length; i++)
            {
                if (nextArgShouldBeArrayLength)
                {
                    if (args[i].GetType() == typeof(int))
                    {
                        var val = (int)args[i];
                        if (nextArgLength != val)
                        {
                            HyperlightException.LogAndThrowException<ArgumentException>($"Array length {val} does not match expected length {nextArgLength}.", GetType().Name);
                        }
                        var pValue = hlint.Createhlint(builder, val);
                        parameters[i] = Parameter.CreateParameter(builder, ParameterValue.hlint, pValue.Value);
                        nextArgShouldBeArrayLength = false;
                        nextArgLength = 0;
                    }
                    else
                    {
                        HyperlightException.LogAndThrowException<ArgumentException>($"Argument {i} is not an int, the length of the array must follow the array itself", GetType().Name);
                    }
                }
                else
                {
                    if (args[i].GetType() == typeof(int))
                    {
                        var val = (int)args[i];
                        var pValue = hlint.Createhlint(builder, val);
                        parameters[i] = Parameter.CreateParameter(builder, ParameterValue.hlint, pValue.Value);
                    }
                    else if (args[i].GetType() == typeof(long))
                    {
                        var val = (long)args[i];
                        var pValue = hllong.Createhllong(builder, val);
                        parameters[i] = Parameter.CreateParameter(builder, ParameterValue.hllong, pValue.Value);
                    }
                    else if (args[i].GetType() == typeof(string))
                    {
                        var val = builder.CreateString((string)args[i]);
                        var pValue = hlstring.Createhlstring(builder, val);
                        parameters[i] = Parameter.CreateParameter(builder, ParameterValue.hlstring, pValue.Value);
                    }
                    else if (args[i].GetType() == typeof(bool))
                    {
                        var val = (bool)args[i];
                        var pValue = hlbool.Createhlbool(builder, val);
                        parameters[i] = Parameter.CreateParameter(builder, ParameterValue.hlbool, pValue.Value);
                    }
                    else if (args[i].GetType() == typeof(byte[]))
                    {
                        var val = (byte[])args[i];
                        var vec = hlvecbytes.CreateValueVector(builder, val);
                        var pValue = hlvecbytes.Createhlvecbytes(builder, vec);
                        parameters[i] = Parameter.CreateParameter(builder, ParameterValue.hlvecbytes, pValue.Value);
                        nextArgShouldBeArrayLength = true;
                        nextArgLength = val.Length;
                    }
                    else
                    {
                        HyperlightException.LogAndThrowException<ArgumentException>("Unsupported parameter type", GetType().Name);
                    }
                }
            }


            if (nextArgShouldBeArrayLength)
            {
                HyperlightException.LogAndThrowException<ArgumentException>("Array length must be specified", GetType().Name);
            }

            var typeofReturnValue = Type.GetTypeFromHandle(returnType);

            var expectedReturnType = ReturnType.hlvoid;

            // TODO need to figure out how to detect and handle void return type

            if (typeofReturnValue.IsAssignableFrom(typeof(Int32)))
            {
                expectedReturnType = ReturnType.hlint;
            }
            else if (typeofReturnValue.IsAssignableFrom(typeof(Int64)))
            {
                expectedReturnType = ReturnType.hllong;
            }
            else if (typeofReturnValue.IsAssignableFrom(typeof(String)))
            {
                expectedReturnType = ReturnType.hlstring;
            }
            else if (typeofReturnValue.IsAssignableFrom(typeof(Boolean)))
            {
                expectedReturnType = ReturnType.hlbool;
            }
            else if (typeofReturnValue.IsAssignableFrom(typeof(byte[])))
            {
                expectedReturnType = ReturnType.hlsizeprefixedbuffer;
            }
            else if (typeofReturnValue == typeof(void))
            {
                expectedReturnType = ReturnType.hlvoid;
            }
            else
            {
                HyperlightException.LogAndThrowException<ArgumentException>($"Unsupported return type {typeofReturnValue.Name}", GetType().Name);
            }

            var parametersVector = FunctionCall.CreateParametersVector(builder, parameters);
            var functionCallType = FunctionCallType.guest;
            var guestFunctionCall = FunctionCall.CreateFunctionCall(builder, funcName, parametersVector, functionCallType, expectedReturnType);
            FunctionCall.FinishSizePrefixedFunctionCallBuffer(builder, guestFunctionCall);
            var buffer = builder.SizedByteArray();

            unsafe
            {
                fixed (byte* guestFunctionCallBuffferPtr = buffer)
                {
                    using var resultHdlWrapper = new Handle(
                    this.ctxWrapper,
                    mem_mgr_write_guest_function_call(
                        this.ctxWrapper.ctx,
                        this.memMgrHdl.handle,
                        (IntPtr)guestFunctionCallBuffferPtr),
                        true
                    );
                }
            }


        }

        internal void WriteHostFunctionDetails(Dictionary<string, HostMethodInfo> hostFunctionInfo)
        {
            FlatBufferBuilder builder = new(1024);
            var hostFunctionDefinitions = new Offset<HostFunctionDefinition>[hostFunctionInfo.Count];
            var i = 0;

            foreach (var hostFunction in hostFunctionInfo)
            {
                var methodInfo = hostFunction.Value.methodInfo;

                var functionName = builder.CreateString(hostFunction.Key);

                var returnType = ReturnType.hlint;
                // TODO: Add support for additional return types
                if (methodInfo.ReturnType == typeof(int) || methodInfo.ReturnType == typeof(uint))
                {
                    returnType = ReturnType.hlint;
                }
                else if (methodInfo.ReturnType == typeof(long) || methodInfo.ReturnType == typeof(IntPtr))
                {
                    returnType = ReturnType.hllong;
                }
                else if (methodInfo.ReturnType == typeof(void))
                {
                    returnType = ReturnType.hlvoid;
                }
                else
                {
                    HyperlightException.LogAndThrowException<ArgumentException>($"Only void int long or IntPtr return types are supported: Name {hostFunction.Key} Return Type {methodInfo.ReturnType.Name} ", GetType().Name);
                }

                VectorOffset parameterTypeVec = new();

                if (methodInfo.GetParameters().Length > 0)
                {
                    var parameterTypes = new ParameterType[methodInfo.GetParameters().Length];
                    ParameterType? parameterType = null;
                    var p = 0;
                    // TODO: add support for additional types.
                    foreach (var parameterInfo in methodInfo.GetParameters())
                    {
                        switch (parameterInfo.ParameterType.Name)
                        {
                            case "Int32":
                                parameterType = ParameterType.hlint;
                                break;
                            case "String":
                                parameterType = ParameterType.hlstring;
                                break;
                            default:
                                HyperlightException.LogAndThrowException<ArgumentException>($"Only int and string parameters are supported: Name {hostFunction.Key} Parameter Type {parameterInfo.ParameterType.Name} ", GetType().Name);
                                break;
                        }
                        parameterTypes[p] = parameterType!.Value;
                        p++;
                    }
                    parameterTypeVec = HostFunctionDefinition.CreateParametersVector(builder, parameterTypes);
                }

                var hostFunctionDefinition = HostFunctionDefinition.CreateHostFunctionDefinition(builder, functionName, parameterTypeVec, returnType);
                hostFunctionDefinitions[i] = hostFunctionDefinition;
                i++;
            }
            var hostFunctionDefinitionsVector = HostFunctionDefinition.CreateSortedVectorOfHostFunctionDefinition(builder, hostFunctionDefinitions);
            var hostFunctionDetails = HostFunctionDetails.CreateHostFunctionDetails(builder, hostFunctionDefinitionsVector);
            builder.FinishSizePrefixed(hostFunctionDetails.Value);
            var buffer = builder.SizedByteArray();

            unsafe
            {
                fixed (byte* hostFunctionDetailsBuffferPtr = buffer)
                {
                    using var resultHdlWrapper = new Handle(
                    this.ctxWrapper,
                    mem_mgr_write_host_function_details(
                        this.ctxWrapper.ctx,
                        this.memMgrHdl.handle,
                        (IntPtr)hostFunctionDetailsBuffferPtr),
                        true
                    );
                }
            }
        }

        internal void WriteResponseFromHostMethodCall(
            Type type,
            object? returnValue
        )
        {
            using var funcCallRes = HostFunctionCallResultWrapper.FromObject(
                this.ctxWrapper,
                type,
                returnValue
            );
            using var hdl = new Handle(
                this.ctxWrapper,
                mem_mgr_write_response_from_host_method_call(
                    this.ctxWrapper.ctx,
                    this.memMgrHdl.handle,
                    funcCallRes.HandleWrapper.handle
                ),
                true
            );
        }


        internal FunctionCall GetHostFunctionCall()
        {
            using var resultHdlWrapper = new Handle(
                this.ctxWrapper,
                mem_mgr_get_host_function_call(
                    this.ctxWrapper.ctx,
                    this.memMgrHdl.handle
                ),
                true
            );

            if (!resultHdlWrapper.IsHostFunctionCall())
            {
                throw new HyperlightException("mem_mgr_get_host_function_call did not return a FunctionCall");
            }
            return resultHdlWrapper.GetHostFunctionCall();
        }

        internal GuestLogData ReadGuestLogData()
        {
            var rawHdl = mem_mgr_read_guest_log_data(
                this.ctxWrapper.ctx,
                this.memMgrHdl.handle
            );
            using var hdl = new Handle(
                this.ctxWrapper,
                rawHdl,
                true
            );
            if (!hdl.IsGuestLogData())
            {
                HyperlightException.LogAndThrowException(
                    "mem_mgr_read_guest_log_data did not return a Handle referencing a GuestLogData",
                    GetType().Name
                );
            }
            return hdl.GetGuestLogData();
        }

        private void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    this.memMgrHdl.Dispose();
                }
                disposedValue = true;
            }
        }


        ~SandboxMemoryManager()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: false);
        }

        public void Dispose()
        {
            // Do not change this code. Put cleanup code in 'Dispose(bool disposing)' method
            Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }

#pragma warning disable CA1707 // Remove the underscores from member name
#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_set_up_hypervisor_partition(
            NativeContext ctx,
            NativeHandle mgrHdl,
            ulong mem_size
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_peb_address(
            NativeContext ctx,
            NativeHandle mgrHdl,
            ulong memStartAddr
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_snapshot_state(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_restore_state(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_return_value(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_set_outb_address(
            NativeContext ctx,
            NativeHandle mgrHdl,
            ulong addr
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_pointer_to_dispatch_function(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_has_host_exception(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_host_exception_length(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_host_exception_data(
            NativeContext ctx,
            NativeHandle mgrHdl,
            IntPtr exceptionDataPtr,
            int exceptionDataLen
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_write_outb_exception(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle guestErrorMsgHdl,
            NativeHandle hostExceptionDataHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_guest_error(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_entrypoint_offset(
            NativeContext ctx,
            NativeHandle memMgrHdl
        );
        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_shared_memory(
            NativeContext ctx,
            NativeHandle memMgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_load_addr(
            NativeContext ctx,
            NativeHandle memMgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]

        private static extern NativeHandle mem_mgr_write_guest_function_call(
            NativeContext ctx,
            NativeHandle mgrHdl,
            IntPtr guestFunctionCallBuffferPtr
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_write_host_function_details(
            NativeContext ctx,
            NativeHandle mgrHdl,
            IntPtr hostFunctionDetailsBuffferPtr
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_write_response_from_host_method_call(
            NativeContext ctx,
            NativeHandle mgrHdl,
            NativeHandle typeNameHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_mem_size(
            NativeContext ctx,
            NativeHandle memMgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_guard_page_offset(
            NativeContext ctx,
            NativeHandle memMgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_host_function_call(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_get_function_call_result(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

        [DllImport("hyperlight_capi", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle mem_mgr_read_guest_log_data(
            NativeContext ctx,
            NativeHandle mgrHdl
        );

#pragma warning restore CA1707 // Remove the underscores from member name
#pragma warning restore CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
    }
}
