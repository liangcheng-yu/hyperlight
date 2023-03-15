using System;
using System.Collections.Immutable;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Threading;
using Hyperlight.Core;
using Hyperlight.Generated;
using Hyperlight.Hypervisors;
using Hyperlight.Native;
using Hyperlight.Wrapper;
using HyperlightDependencies;
using Microsoft.Extensions.Logging;

namespace Hyperlight
{
    enum OutBAction
    {
        Log = 99,
        WriteOutput = 100,
        CallFunction = 101,
        Abort = 102,
    }

    [Flags]
    public enum SandboxRunOptions
    {
        None = 0,
        RunInProcess = 1,
        RecycleAfterRun = 2,
        RunFromGuestBinary = 4,
    }
    public class Sandbox : IDisposable, ISandboxRegistration
    {
        public static AsyncLocal<string> CorrelationId { get; } = new AsyncLocal<string>();
        readonly Context context;
        readonly string? fixedCorrelationId;
        static bool IsWindows => RuntimeInformation.IsOSPlatform(OSPlatform.Windows);
        public static bool IsLinux => RuntimeInformation.IsOSPlatform(OSPlatform.Linux);
        public static bool IsSupportedPlatform => IsLinux || IsWindows;
        Hypervisor? hyperVisor;
        GCHandle? gCHandle;
        byte[]? stackGuard;
        readonly string guestBinaryPath;
        readonly HyperLightExports hyperLightExports;
        readonly Core.SandboxMemoryManager sandboxMemoryManager;
        readonly bool initialised;
        readonly bool recycleAfterRun;
        readonly bool runFromProcessMemory;
        readonly bool runFromGuestBinary;
        bool didRunFromGuestBinary;
        const int IS_RUNNING_FROM_GUEST_BINARY = 1;
        // this is passed as a ref to several functions below,
        // therefore cannot be readonly
        static int isRunningFromGuestBinary;
        readonly StringWriter? writer;
        readonly HyperlightGuestInterfaceGlue guestInterfaceGlue;
        private bool disposedValue; // To detect redundant calls
        HyperlightPEB? hyperlightPEB;

        unsafe delegate* unmanaged<IntPtr, ulong, int> callEntryPoint;

        // Platform dependent delegate for callbacks from native code when native code is calling 'outb' functionality
        // On Linux, delegates passed from .NET core to native code expect arguments to be passed RDI, RSI, RDX, RCX.
        // On Windows, the expected order starts with RCX, RDX.  Our native code assumes this Windows calling convention
        // so 'port' is passed in RCX and 'value' is passed in RDX.  When run in Linux with in-process set to true (and the
        // guest is a PE file), we have an alternate callback that will take RCX and RDX in the different positions and pass 
        // it to the HandleOutb method correctly
        // The registers are different because when .NET is running on linux it is using one call pattern, and the guest binary (PE file) is compiled for a different call pattern.
        // The wonky CallOutb_Linux delegate is for when we are crossing the call pattern streams.

        delegate void CallOutb_Windows(ushort port, byte value);
        delegate void CallOutb_Linux(int unused1, int unused2, byte value, ushort port);
        delegate void CallDispatchFunction();

        // 0 No calls are executing
        // 1 Call guest is executing
        // 2 Dynamic Method is executing standalone
        ulong executingGuestCall;

        readonly ulong rsp;

        int countRunCalls;

        readonly Func<string> GetCorrelationId;

        /// <summary>
        /// Returns the maximum number of partitions per process, on windows its the mximum number of processes that can be handled by the HyperVSurrogateProcessManager , on Linux its not fixed and dependent on resources.
        /// </summary>

        public static int MaxPartitionsPerProcess => IsWindows ? HyperVSurrogateProcessManager.NumberOfProcesses : -1;

        /// <summary>
        /// Create a new Sandbox. Note that you may also use the
        /// SandboxBuilder class to construct Sandboxes
        /// to this constructor.
        /// </summary>
        /// <param name="guestBinaryPath">
        /// The path location of the binary to run inside the sandbox
        /// </param>
        /// <param name="runOptions">
        /// Optional Options with which to configure the runtime. Default is to run in HyperVisor.
        /// </param>
        /// <param name="initFunction">
        /// Optional function to execute on init
        /// </param>
        /// <param name="writer">
        /// Optional writer with which to write outb data
        /// </param>
        /// <param name="correlationId">
        /// Optional correlationId to use for logging
        /// </param>
        /// <param name="errorMessageLogger">
        /// Optional ILogger to use for logging
        /// </param>
        /// <param name="sandboxMemoryConfiguration">
        /// Optional memory configuration with which to create this sandbox
        /// </param>
        /// <param name="getCorrelationIdFunc">
        /// Optional function called by the Sandbox to get the correlationId
        /// </param>
        /// <exception cref="PlatformNotSupportedException">
        /// If a sandbox is constructed on a platform on which it 
        /// can't currently run
        /// </exception>
        /// <exception cref="ArgumentException">
        /// If any one or more of the following conditions are true:
        /// * guestBinaryPath is a reference to a non-existent file
        /// * runOptions.RunFromGuestBinary and runOptions.RecycleAfterRun
        /// are both set 
        /// * a suitable Hypervisor is not found
        /// </exception>
        public Sandbox(
            string guestBinaryPath,
            SandboxRunOptions? runOptions,
            Action<ISandboxRegistration>? initFunction = null,
            StringWriter? writer = null,
            string? correlationId = null,
            ILogger? errorMessageLogger = null,
            SandboxMemoryConfiguration? sandboxMemoryConfiguration = null,
            Func<string>? getCorrelationIdFunc = null
        )
        {
            if (!string.IsNullOrEmpty(correlationId))
            {
                fixedCorrelationId = correlationId;
            }

            // Set the function to get the correlationId to the one that was provided , if this is null then use the default function

            GetCorrelationId = getCorrelationIdFunc ?? GetDefaultCorrelationId;

            // Use the function to get the correlationId

            UpdateCorrelationId();
            var memCfg = sandboxMemoryConfiguration ?? new SandboxMemoryConfiguration();
            this.context = new Context(CorrelationId.Value!);

            if (!IsSupportedPlatform)
            {
                HyperlightException.LogAndThrowException<PlatformNotSupportedException>("Hyperlight is not supported on this platform", GetType().Name);
            }

            runOptions ??= SandboxRunOptions.None;

            if (!File.Exists(guestBinaryPath))
            {
                HyperlightException.LogAndThrowException<ArgumentException>($"Cannot find file {guestBinaryPath} to load into hyperlight", GetType().Name);
            }
            this.writer = writer;
            this.guestBinaryPath = guestBinaryPath;

            this.recycleAfterRun = (runOptions & SandboxRunOptions.RecycleAfterRun) == SandboxRunOptions.RecycleAfterRun;
            this.runFromProcessMemory = (runOptions & SandboxRunOptions.RunInProcess) == SandboxRunOptions.RunInProcess ||
                                        (runOptions & SandboxRunOptions.RunFromGuestBinary) == SandboxRunOptions.RunFromGuestBinary;
            this.runFromGuestBinary = (runOptions & SandboxRunOptions.RunFromGuestBinary) == SandboxRunOptions.RunFromGuestBinary;

            // TODO: should we make this work?
            if (recycleAfterRun && runFromGuestBinary)
            {
                HyperlightException.LogAndThrowException<ArgumentException>("Cannot run from guest binary and recycle after run at the same time", GetType().Name);
            }

            this.guestInterfaceGlue = new HyperlightGuestInterfaceGlue(this);


            HyperlightLogger.SetLogger(errorMessageLogger);
            {
                var (mgr, didRunFromGuestBinary) = LoadGuestBinary(
                    this.context,
                    memCfg,
                    guestBinaryPath,
                    runFromProcessMemory,
                    runFromGuestBinary
                );
                this.sandboxMemoryManager = mgr;
                this.didRunFromGuestBinary = didRunFromGuestBinary;
            }
            SetUpHyperLightPEB();
            SetUpStackGuard();
            rsp = 0;
            // If we are NOT running from process memory, we have to setup a Hypervisor partition
            if (!runFromProcessMemory)
            {
                if (!IsHypervisorPresent())
                {
                    HyperlightException.LogAndThrowException<ArgumentException>("Hypervisor not found", GetType().Name);
                }
                rsp = SetUpHyperVisorPartition();
            }

            hyperLightExports = new HyperLightExports();
            ExposeAndBindMembers(hyperLightExports);

            Initialise();

            initFunction?.Invoke(this);

            if (recycleAfterRun)
            {
                sandboxMemoryManager.SnapshotState();
            }

            if (!runFromProcessMemory)
            {
                hyperVisor!.ResetRSP(rsp);
            }

            initialised = true;

        }

        // Default function to get a correlationid ,if a correlationId was provided in the constructor
        // then use it otherwise generate a new one for each invocation of the function.
        string GetDefaultCorrelationId()
        {
            return fixedCorrelationId ?? Guid.NewGuid().ToString("N");
        }

        // Update the correlationId if the function provided  returns null or throws an exception then use the default function.
        internal void UpdateCorrelationId()
        {
            string result;
            try
            {
                result = GetCorrelationId() ?? GetDefaultCorrelationId();
            }
#pragma warning disable CA1031 // Intentional to catch alll exceptions here as we dont want to crash just because we cannot get a correlationId from the function.
            catch (Exception ex)
#pragma warning restore CA1031
            {
                result = GetDefaultCorrelationId();
                HyperlightLogger.LogError($"Exception thrown tyring to get new CorrelationId {ex.GetType().Name} {ex.Message}", nameof(GetCorrelationId));
            }
            if (this.context != null && this.context.CorrelationId != result)
            {
                this.context.CorrelationId = result;
            }
            CorrelationId.Value = result;
        }
        internal object DispatchCallFromHost(string functionName, object[] args)
        {
            var pDispatchFunction = sandboxMemoryManager.GetPointerToDispatchFunction();

            if (pDispatchFunction == 0)
            {
                HyperlightException.LogAndThrowException<ArgumentException>($"{nameof(pDispatchFunction)} is null", GetType().Name);
            }

            sandboxMemoryManager.WriteGuestFunctionCallDetails(functionName, args);

            if (runFromProcessMemory)
            {
                var callDispatchFunction = Marshal.GetDelegateForFunctionPointer<CallDispatchFunction>((IntPtr)pDispatchFunction);
                callDispatchFunction();
            }
            else
            {
                hyperVisor!.DispatchCallFromHost(pDispatchFunction);
            }

            if (!CheckStackGuard())
            {
                HyperlightException.LogAndThrowException<StackOverflowException>($"Calling {functionName}", GetType().Name);
            }

            CheckForGuestError();

            return sandboxMemoryManager.GetReturnValue();
        }

        internal void HandleMMIOExit()
        {
            if (!CheckStackGuard())
            {
                HyperlightException.LogAndThrowException<StackOverflowException>($"Calling HandleMMIOExit", GetType().Name);
            }

        }

        public void CheckForGuestError()
        {
            (ErrorCode ErrorCode, string? Message) guestError = sandboxMemoryManager.GetGuestError();

            switch (guestError.ErrorCode)
            {
                case ErrorCode.NoError:
                    break;
                case ErrorCode.OutbError:
                    {
                        var exception = sandboxMemoryManager.GetHostException();
                        if (exception != null)
                        {
                            if (exception.InnerException != null)
                            {
                                HyperlightLogger.LogError($"Exception from Host With Inner exception {exception.GetType().Name} {exception.Message}", GetType().Name);
                                HyperlightLogger.LogError($"Rethrowing Inner exception from Host {exception.InnerException.GetType().Name} {exception.InnerException.Message}", GetType().Name);
                                throw exception.InnerException;
                            }
                            HyperlightLogger.LogError($"Rethrowing exception from Host {exception.GetType().Name} {exception.Message}", GetType().Name);
                            throw exception;
                        }
                        HyperlightException.LogAndThrowException("OutB Error", GetType().Name);
                        break;
                    }
                case ErrorCode.StackOverflow:
                    {
                        HyperlightException.LogAndThrowException<StackOverflowException>($"Guest Error", GetType().Name);
                        break;
                    }
                default:
                    {
                        var message = $"{guestError.ErrorCode}:{guestError.Message}";
                        HyperlightException.LogAndThrowException(message, GetType().Name);
                        break;
                    }
            }
        }

        private static (Core.SandboxMemoryManager, bool) LoadGuestBinary(
            Context ctx,
            SandboxMemoryConfiguration memCfg,
            string guestBinaryPath,
            bool runFromProcessMemory,
            bool runFromGuestBinary
        )
        {
            var peInfo = new PEInfo(ctx, guestBinaryPath);
            bool didRunFromGuestBinary = false;
            try
            {
                if (runFromGuestBinary)
                {
                    if (!IsWindows)
                    {
                        // If not on Windows runFromBinary doesn't mean anything because we cannot use LoadLibrary.
                        HyperlightException.LogAndThrowException<NotSupportedException>(
                            "RunFromBinary is only supported on Windows",
                            MethodBase.GetCurrentMethod()!.DeclaringType!.Name
                        );
                    }

                    // LoadLibrary does not support multiple independent instances of a binary beng loaded 
                    // so we cannot support multiple instances using loadlibrary
                    if (Interlocked.CompareExchange(ref isRunningFromGuestBinary, IS_RUNNING_FROM_GUEST_BINARY, 0) == 0)
                    {
                        didRunFromGuestBinary = true;
                    }
                    else
                    {
                        HyperlightException.LogAndThrowException(
                            "Only one instance of Sandbox is allowed when running from guest binary",
                            MethodBase.GetCurrentMethod()!.DeclaringType!.Name
                        );
                    }

                    var memMgr = Core.SandboxMemoryManager.LoadGuestBinaryUsingLoadLibrary(
                        ctx,
                        memCfg,
                        guestBinaryPath,
                        runFromProcessMemory
                    );
                    return (memMgr, didRunFromGuestBinary);
                }
                else
                {
                    var memMgr = Core.SandboxMemoryManager.LoadGuestBinaryIntoMemory(
                        ctx,
                        memCfg,
                        guestBinaryPath,
                        runFromProcessMemory
                    );
                    return (memMgr, didRunFromGuestBinary);
                }
            }
            finally
            {
                peInfo.Dispose();
            }
        }

        void SetUpStackGuard()
        {
            stackGuard = RandomNumberGenerator.GetBytes(16);
            sandboxMemoryManager.SetStackGuard(stackGuard);
        }

        bool CheckStackGuard()
        {
            return sandboxMemoryManager.CheckStackGuard(stackGuard);
        }

        void SetUpHyperLightPEB()
        {
            hyperlightPEB = sandboxMemoryManager.SetUpHyperLightPEB();
            CreateHyperlightPEBInMemory();
        }

        private void CreateHyperlightPEBInMemory()
        {
            UpdateFunctionMap();
            hyperlightPEB!.Create();
        }

        private void UpdateHyperlightPEBInMemory()
        {
            UpdateFunctionMap();
            hyperlightPEB!.Update();
        }

        private void UpdateFunctionMap()
        {
            foreach (var mi in guestInterfaceGlue.MapHostFunctionNamesToMethodInfo.Values)
            {

                // Dont add functions that already exist in the PEB
                // TODO: allow overloaded functions

                if (hyperlightPEB!.FunctionExists(mi.methodInfo.Name))
                {
                    continue;
                }

                string returntype = string.Empty;
                // TODO: Add support for void return types
                if (mi.methodInfo.ReturnType == typeof(int) || mi.methodInfo.ReturnType == typeof(uint))
                {
                    returntype = "i";
                }
                else if (mi.methodInfo.ReturnType == typeof(long) || mi.methodInfo.ReturnType == typeof(IntPtr))
                {
                    returntype = "I";
                }
                else
                {
                    HyperlightException.LogAndThrowException<ArgumentException>("Only int long or IntPtr return types are supported", GetType().Name);
                }

                var parameterSignature = "";
                foreach (var pi in mi.methodInfo.GetParameters())
                {
                    if (pi.ParameterType == typeof(int))
                        parameterSignature += "i";
                    else if (pi.ParameterType == typeof(string))
                        parameterSignature += "$";
                    else
                        HyperlightException.LogAndThrowException<ArgumentException>("Only int and string parameters are supported", GetType().Name);
                }

                hyperlightPEB.AddFunction(mi.methodInfo.Name, $"({parameterSignature}){returntype}", 0);
            }
        }

        public ulong SetUpHyperVisorPartition()
        {
            var rsp = sandboxMemoryManager.SetUpHyperVisorPartition();

            var memSize = this.sandboxMemoryManager.Size;
            var baseAddr = SandboxMemoryLayout.BaseAddress;
            var pml4Addr = SandboxMemoryLayout.PML4GuestAddress;
            var entryPoint = this.sandboxMemoryManager.EntryPoint;

            // ensure PML4 < entry point < stack pointer
            checkSequence(
                memSize,
                baseAddr,
                new (string, ulong)[]{
                        ("PML4Addr", pml4Addr),
                        ("EntryPoint", entryPoint),
                        ("RSP", rsp)
                }.ToImmutableList()
            );
            if (IsLinux)
            {
                if (LinuxHyperV.IsHypervisorPresent())
                {
                    hyperVisor = new HyperVOnLinux(
                        this.context,
                        sandboxMemoryManager.SourceAddress,
                        pml4Addr,
                        memSize,
                        entryPoint,
                        rsp,
                        HandleOutb,
                        HandleMMIOExit
                    );
                }
                else if (LinuxKVM.IsHypervisorPresent())
                {
                    hyperVisor = new KVM(
                        sandboxMemoryManager.SourceAddress,
                        pml4Addr,
                        memSize,
                        entryPoint,
                        rsp,
                        HandleOutb,
                        HandleMMIOExit
                    );
                }
                else
                {
                    // Should never get here
                    HyperlightException.LogAndThrowException<NotSupportedException>("Only KVM and HyperV are supported on Linux", GetType().Name);
                }
            }
            else if (IsWindows)
            {
                hyperVisor = new HyperV(
                    sandboxMemoryManager.SourceAddress,
                    pml4Addr,
                    memSize,
                    entryPoint,
                    rsp,
                    HandleOutb,
                    HandleMMIOExit
                );
            }
            else
            {
                // Should never get here
                HyperlightException.LogAndThrowException<NotSupportedException>("Only supported on Linux and Windows", GetType().Name);
            }

            return rsp;
        }

        private void Initialise()
        {
            var returnValue = 0;
            var seedBytes = new byte[8];
            using (var randomNumberGenerator = RandomNumberGenerator.Create())
            {
                randomNumberGenerator.GetBytes(seedBytes);
            }
            var seed = BitConverter.ToUInt64(seedBytes);
            var pebAddress = (IntPtr)sandboxMemoryManager.GetPebAddress();

            if (runFromProcessMemory)
            {
                if (IsLinux)
                {
                    // This code is unstable, it causes segmentation faults so for now we are throwing an exception if we try to run in process in Linux
                    // I think this is due to the fact that the guest binary is built for windows
                    // x64 compilation for windows uses fastcall which is different on windows and linux
                    // dotnet will default to the calling convention for the platform that the code is running on
                    // so we need to set the calling convention to the one that the guest binary is built for (windows x64 https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention?view=msvc-170)
                    // on linux however, this isn't possible (https://docs.microsoft.com/en-us/dotnet/api/system.runtime.interopservices.callingconvention?view=net-6.0 )
                    // Alternatives:
                    // 1. we need to build the binary for windows and linux and then run the correct version for the platform that we are running on
                    // 2. alter the calling convention of the guest binary and then tell dotnet to use that calling convention
                    // the only option for this seems to be vectorcall https://docs.microsoft.com/en-us/cpp/cpp/vectorcall?view=msvc-170 (cdecl and stdcall are not possible using CL on x64 platform))    
                    // vectorcall is not supported by dotnet  (https://github.com/dotnet/runtime/issues/8300) 
                    // 3. write our own code to correct the calling convention
                    // 4. write epilog/prolog code in the guest binary.     
                    // also see https://www.agner.org/optimize/calling_conventions.pdf
                    // and https://eli.thegreenplace.net/2011/09/06/stack-frame-layout-on-x86-64/
                    // 
                    HyperlightException.LogAndThrowException<NotSupportedException>("Cannot run in process on Linux", GetType().Name);

#pragma warning disable CS0162 // Unreachable code detected - this is temporary until the issue above is fixed.
                    var callOutB = new CallOutb_Linux((_, _, value, port) => HandleOutb(port, value));
                    gCHandle = GCHandle.Alloc(callOutB);
                    sandboxMemoryManager.SetOutBAddress((long)Marshal.GetFunctionPointerForDelegate<CallOutb_Linux>(callOutB));
                    CallEntryPoint(pebAddress, seed);
#pragma warning restore CS0162 // Unreachable code detected

                }
                else if (IsWindows)
                {
                    var callOutB = new CallOutb_Windows((port, value) => HandleOutb(port, value));

                    gCHandle = GCHandle.Alloc(callOutB);
                    sandboxMemoryManager.SetOutBAddress((long)Marshal.GetFunctionPointerForDelegate<CallOutb_Windows>(callOutB));
                    CallEntryPoint(pebAddress, seed);
                }
                else
                {
                    // Should never get here
                    HyperlightException.LogAndThrowException<NotSupportedException>("Can only run in process on Linux and Windows", GetType().Name);
                }
            }
            else
            {
                hyperVisor!.Initialise(pebAddress, seed);
            }

            if (!CheckStackGuard())
            {
                HyperlightException.LogAndThrowException<StackOverflowException>("Init Function Failed", GetType().Name);
            }

            returnValue = sandboxMemoryManager.GetReturnValue();

            if (returnValue != 0)
            {
                CheckForGuestError();
                HyperlightException.LogAndThrowException($"Init Function Failed with error code:{returnValue}", GetType().Name);
            }
        }

        unsafe void CallEntryPoint(IntPtr pebAddress, ulong seed)
        {
            callEntryPoint = (delegate* unmanaged<IntPtr, ulong, int>)sandboxMemoryManager.EntryPoint;
            _ = callEntryPoint(pebAddress, seed);
        }

        /// <summary>
        /// Enables the host to call multiple functions in the Guest and have the sandbox state reset at the start of the call
        /// Ensures that only one call can be made concurrently
        /// </summary>
        /// <typeparam name="T">The return type of the function</typeparam>
        /// <param name="func">The function to be executed</param>
        /// <returns>T</returns>
        /// <exception cref="ArgumentNullException">func is null</exception>
        /// <exception cref="HyperlightException">a call to the guest is already in progress</exception>
        public T CallGuest<T>(Func<T> func)
        {
            HyperlightException.ThrowIfNull(func, nameof(func), GetType().Name);
            var shouldRelease = false;
            try
            {
                if (Interlocked.CompareExchange(ref executingGuestCall, 1, 0) != 0)
                {
                    HyperlightException.LogAndThrowException("Guest call already in progress", GetType().Name);
                }
                shouldRelease = true;
                ResetState();
                return func();
            }
            finally
            {
                if (shouldRelease)
                {
                    Interlocked.Exchange(ref executingGuestCall, 0);
                }
            }
        }

        /// <summary>
        /// This method is called by DynamicMethods generated to call guest functions.
        /// It first checks to see if the sadnbox has been initialised yet or if there is a CallGuest Method call in progress, if so it just
        /// returns false as there is no need to check state
        /// </summary>
        /// <returns></returns>
        /// <exception cref="HyperlightException"></exception>
        internal bool EnterDynamicMethod()
        {
            // If at least one of the following are true, we don't need to set
            // state:
            // 1. we haven't been initialised yet
            // 2. we are finished or invoked inside CallGuest<T>
            if (!initialised || Interlocked.Read(ref executingGuestCall) == 1)
            {
                return false;
            }

            if ((Interlocked.CompareExchange(ref executingGuestCall, 2, 0)) != 0)
            {
                HyperlightException.LogAndThrowException("Guest call already in progress", GetType().Name);
            }
            return true;
        }

        internal void ExitDynamicMethod(bool shouldRelease)
        {
            if (shouldRelease)
            {
                Interlocked.Exchange(ref executingGuestCall, 0);
            }
        }

        internal void ResetState()
        {

            if (countRunCalls > 0 && !recycleAfterRun)
            {
                HyperlightException.LogAndThrowException<ArgumentException>("You must set option RecycleAfterRun when creating the Sandbox if you need to call a function in the guest more than once", GetType().Name);
            }

            if (recycleAfterRun)
            {
                sandboxMemoryManager.RestoreState();
                if (!runFromProcessMemory)
                {
                    hyperVisor!.ResetRSP(rsp);
                }
            }

            countRunCalls++;

        }

        internal void HandleOutb(ushort port, byte _)
        {
            try
            {
                switch ((OutBAction)port)
                {
                    case OutBAction.CallFunction:
                        {

                            var methodName = sandboxMemoryManager.GetHostCallMethodName();
                            HyperlightException.ThrowIfNull(methodName, GetType().Name);

                            if (!guestInterfaceGlue.MapHostFunctionNamesToMethodInfo.ContainsKey(methodName))
                            {
                                HyperlightException.LogAndThrowException<ArgumentException>($"{methodName}, Could not find host method name.", GetType().Name);
                            }

                            var mi = guestInterfaceGlue.MapHostFunctionNamesToMethodInfo[methodName];
                            var parameters = mi.methodInfo.GetParameters();
                            var args = sandboxMemoryManager.GetHostCallArgs(parameters);
                            var returnFromHost = guestInterfaceGlue.DispatchCallFromGuest(methodName, args);
                            sandboxMemoryManager.WriteResponseFromHostMethodCall(mi.methodInfo.ReturnType, returnFromHost);
                            break;

                        }
                    case OutBAction.WriteOutput:
                        {
                            var str = sandboxMemoryManager.ReadStringOutput();
                            if (this.writer != null)
                            {
                                writer.Write(str);
                            }
                            else
                            {
                                var oldColor = Console.ForegroundColor;
                                Console.ForegroundColor = ConsoleColor.Green;
                                Console.Write(str);
                                Console.ForegroundColor = oldColor;
                            }
                            break;
                        }
                    case OutBAction.Log:
                        {
                            var guestLogData = sandboxMemoryManager.ReadGuestLogData();
                            HyperlightLogger.Log(guestLogData.LogLevel, guestLogData.Message, guestLogData.Source, null, guestLogData.Caller, guestLogData.SourceFile, guestLogData.Line);
                            break;
                        }
                    case OutBAction.Abort:
                        {
                            // TODO we should do something different here , maybe change the function to return a value to indicate that the guest aborted
                            // throwing an exception here will cause the host to write an outb exception and this results in the guest being invoked again when it should not be
                            // as an abort signals that the guest should not be used again.
                            throw new HyperlightException("Guest Aborted");
                        }
                }
            }
#pragma warning disable CA1031 // Intentional to catch alll exceptions here as they are serilaised across the managed/unmanaged boundary and we don't want to lose the exception information
            catch (Exception ex)
#pragma warning restore CA1031
            {
                // Call function can throw TargetInvocationException, we need to unwrap the inner exception
                if (ex is TargetInvocationException && ex.InnerException != null)
                {
                    sandboxMemoryManager.WriteOutbException(ex.InnerException, port);
                }
                else
                {
                    sandboxMemoryManager.WriteOutbException(ex, port);
                }
            }
        }

        public void ExposeHostMethods(Type type)
        {
            HyperlightException.ThrowIfNull(type, nameof(type), GetType().Name);
            guestInterfaceGlue.ExposeAndBindMembers(type);
            UpdateHyperLightPEB();
        }

        public void ExposeAndBindMembers(object instance)
        {
            HyperlightException.ThrowIfNull(instance, nameof(instance), GetType().Name);
            guestInterfaceGlue.ExposeAndBindMembers(instance);
            UpdateHyperLightPEB();
        }

        public void BindGuestFunction(string delegateName, object instance)
        {
            HyperlightException.ThrowIfNull(instance, nameof(instance), GetType().Name);
            guestInterfaceGlue.BindGuestFunctionToDelegate(delegateName, instance);
        }

        public void ExposeHostMethod(string methodName, object instance)
        {
            HyperlightException.ThrowIfNull(instance, nameof(instance), GetType().Name);
            guestInterfaceGlue.ExposeHostMethod(methodName, instance);
            UpdateHyperLightPEB();
        }

        public void ExposeHostMethod(string methodName, Type type)
        {
            HyperlightException.ThrowIfNull(type, nameof(type), GetType().Name);
            guestInterfaceGlue.ExposeHostMethod(methodName, type);
            UpdateHyperLightPEB();
        }

        private void UpdateHyperLightPEB()
        {
            if (recycleAfterRun && initialised)
            {
                sandboxMemoryManager.RestoreState();
            }
            UpdateHyperlightPEBInMemory();
            if (recycleAfterRun && initialised)
            {
                sandboxMemoryManager.SnapshotState();
            }
        }

        public static bool IsHypervisorPresent()
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                return LinuxHyperV.IsHypervisorPresent() || LinuxKVM.IsHypervisorPresent();
            }
            else if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                return WindowsHypervisorPlatform.IsHypervisorPresent();
            }
            return false;
        }

        private static ulong checkGuestAddr(ulong memSize, ulong baseAddr, string addrName, ulong addr)
        {
            if (addr < baseAddr)
            {
                HyperlightException.LogAndThrowException(
                    $"{addrName} {addr} < baseAddr {baseAddr}",
                    MethodBase.GetCurrentMethod()!.DeclaringType!.Name
                );
            }
            var offset = addr - baseAddr;
            if (offset > memSize)
            {
                HyperlightException.LogAndThrowException(
                    $"{addrName} {addr} (offset = {offset}) > total shared mem size {memSize}",
                    MethodBase.GetCurrentMethod()!.DeclaringType!.Name
                );
            }
            return addr;
        }

        /// <summary>
        /// Ensure each address in a list is greater than all the 
        /// previous list entries, is greater than the given base address,
        /// and its offset (address - baseAddr) is less than the given
        /// memory size.
        /// </summary>
        /// <param name="memSize">the total memory size</param>
        /// <param name="baseAddr">the base address</param>
        /// <param name="addrs">the list of addresses to check</param>
        private static void checkSequence(ulong memSize, ulong baseAddr, ImmutableList<(string, ulong)> addrs)
        {
            var _ = addrs.Aggregate((prev, cur) =>
            {
                var (prevName, prevAddr) = prev;
                var (curName, curAddr) = cur;
                checkGuestAddr(memSize, baseAddr, curName, curAddr);
                if (prevAddr >= curAddr)
                {
                    HyperlightException.LogAndThrowException(
                    message: $"address {prevName} ({prevAddr}) >= address {curName} ({curAddr})",
                    MethodBase.GetCurrentMethod()!.DeclaringType!.Name
                );
                }
                return cur;
            });
        }

        protected virtual void Dispose(bool disposing)
        {
            if (!disposedValue)
            {
                if (disposing)
                {
                    if (didRunFromGuestBinary)
                    {
                        Interlocked.Decrement(ref isRunningFromGuestBinary);
                    }

                    gCHandle?.Free();

                    sandboxMemoryManager?.Dispose();

                    hyperVisor?.Dispose();
                    this.context.Dispose();

                }

                disposedValue = true;
            }
        }

        public void Dispose()
        {
            // Do not change this code. Put cleanup code in Dispose(bool disposing) above.
            Dispose(true);
            GC.SuppressFinalize(this);
        }
    }
}
