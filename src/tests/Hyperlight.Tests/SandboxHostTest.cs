using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;
using System.Threading.Tasks;
using Hyperlight.Core;
using HyperlightDependencies;
using Microsoft.Extensions.Logging;
using Microsoft.VisualStudio.TestPlatform.ObjectModel.Engine.ClientProtocol;
using Moq;
using Xunit;
using Xunit.Abstractions;

namespace Hyperlight.Tests
{

    public class SandboxHostTest
    {
        private readonly ITestOutputHelper testOutput;
        public const int NUMBER_OF_ITERATIONS = 10;
        public const int NUMBER_OF_PARALLEL_TESTS = 10;
        public const int PAGE_SIZE = 4096;
        // These are the host functions that are exposed automatically by Hyperlight
        private readonly string[] internalFunctions = { "GetStackBoundary", "GetTimeSinceBootMicrosecond" };

        private bool IsRunningOnKVM;

        public SandboxHostTest(ITestOutputHelper testOutput)
        {
            //TODO: implement skip for this
            Assert.True(Sandbox.IsSupportedPlatform, "Hyperlight Sandbox is not supported on this platform.");

            // This is to ensure that the tests fail if they are run on a runner where we expect a Hypervisor but one is not present.
            var expectHyperV = bool.TryParse(Environment.GetEnvironmentVariable("HYPERV_SHOULD_BE_PRESENT"), out var result) && result;
            var expectKVM = bool.TryParse(Environment.GetEnvironmentVariable("KVM_SHOULD_BE_PRESENT"), out result) && result;
            var expectWHP = bool.TryParse(Environment.GetEnvironmentVariable("WHP_SHOULD_BE_PRESENT"), out result) && result;

            if (expectHyperV || expectKVM || expectWHP)
            {
                Assert.True(Sandbox.IsHypervisorPresent(), "HyperVisor not present but expected.");
            }

            // This is a hacky way of trying to figure out if we are running on KVM.  We need to do this because the tests that run on different threads are very slow on KVM so we will reduce the number of iterations.
            IsRunningOnKVM = expectKVM;

            this.testOutput = testOutput;
        }

        public class TestData
        {
            public enum ExposeMembersToGuest
            {
                Instance,
                Type,
                TypeAndNull,
                TypeAndInstance,
                InstanceAndNull,
                Null,
                All
            }
            public int ExpectedReturnValue;
            public string ExpectedOutput;
            public string GuestBinaryPath { get; }
            public Type? instanceOrTypeType { get; }
            public int NumberOfIterations { get; }
            public int NumberOfParallelTests { get; }

            public ExposeMembersToGuest ExposeMembers = ExposeMembersToGuest.All;

            public TestData(
                string guestBinaryFileName,
                string expectedOutput = "Hello, World!!\n",
                int? expectedReturnValue = null,
                int numberOfIterations = 0,
                int numberOfParallelTests = 0,
                ExposeMembersToGuest exposeMembers = ExposeMembersToGuest.All
            )
            {
                var path = AppDomain.CurrentDomain.BaseDirectory;
                var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
                Assert.True(File.Exists(guestBinaryPath), $"Cannot find file {guestBinaryPath} to load into hyperlight");
                this.GuestBinaryPath = guestBinaryPath;
                this.ExpectedOutput = expectedOutput;
                this.ExpectedReturnValue = expectedReturnValue ?? expectedOutput.Length;
                NumberOfIterations = numberOfIterations > 0 ? numberOfIterations : NUMBER_OF_ITERATIONS;
                NumberOfParallelTests = numberOfParallelTests > 0 ? numberOfParallelTests : NUMBER_OF_PARALLEL_TESTS;
                this.ExposeMembers = exposeMembers;
            }

            public TestData(
                Type instanceOrTypeType,
                string guestBinaryFileName,
                string expectedOutput = "Hello, World!!\n",
                int? expectedReturnValue = null,
                int numberOfIterations = 0,
                int numberOfParallelTests = 0,
                ExposeMembersToGuest exposeMembers = ExposeMembersToGuest.All
            ) : this(guestBinaryFileName, expectedOutput, expectedReturnValue, numberOfIterations, numberOfParallelTests, exposeMembers)
            {
                this.instanceOrTypeType = instanceOrTypeType;
            }

            public object?[] TestInstanceOrTypes()
            {
                if (instanceOrTypeType != null)
                {
                    return ExposeMembers switch
                    {
                        ExposeMembersToGuest.Instance => new object?[] { GetInstance(instanceOrTypeType) },
                        ExposeMembersToGuest.Type => new object?[] { instanceOrTypeType },
                        ExposeMembersToGuest.TypeAndNull => new object?[] { instanceOrTypeType, null },
                        ExposeMembersToGuest.TypeAndInstance => new object?[] { instanceOrTypeType, GetInstance(instanceOrTypeType) },
                        ExposeMembersToGuest.InstanceAndNull => new object?[] { GetInstance(instanceOrTypeType), null },
                        ExposeMembersToGuest.Null => new object?[] { null },
                        ExposeMembersToGuest.All => new object?[] { instanceOrTypeType, GetInstance(instanceOrTypeType), null },
                        _ => Array.Empty<object?>(),
                    };
                }

                return Array.Empty<object?>();
            }
        }

        public class SimpleTestMembers
        {
#pragma warning disable IDE0049 // Simplify Names
            public Func<String, int>? PrintOutput;
            public Func<String, string>? Echo;
            public Func<byte[], int, byte[]>? GetSizePrefixedBuffer;
#pragma warning restore IDE0049 // Simplify Names
        }

        public class DontExistMembers
        {
#pragma warning disable IDE0049 // Simplify Names
            public Func<int>? ThisIsNotARealFunctionButTheNameIsImportant;
            public Func<int>? SomethingElse;
#pragma warning restore IDE0049 // Simplify Names
        }

        public class CallbackTestMembers
        {
            readonly StringWriter? output;
            public Func<String, int>? PrintOutput;
            public Func<String, int>? GuestMethod;
            public Func<String, int>? GuestMethod1;
            public Action? GuestMethod4;

            public CallbackTestMembers() { }
            public CallbackTestMembers(StringWriter output)
            {
                this.output = output;
            }

            public int HostMethod(string msg)
            {
                return PrintOutput!($"Host Method Received: {msg} from Guest");
            }
            public int HostMethod1(string msg)
            {
                return PrintOutput!($"Host Method 1 Received: {msg} from Guest");
            }
            public void HostMethod4(string msg)
            {
                if (output != null)
                {
                    output.Write(msg);
                }
            }
        }

        public class HostExceptionTestMembers
        {
            public Func<string, int>? CallErrorMethod;
            public int ErrorMethod(string msg)
            {
                throw new HyperlightException(HyperlightException.GetExceptionMessage(msg, GetType().Name));
            }
        }

        public class NoExposedMembers { }

        public class ExposedMembers
        {
            public int GetOne() => 1;
            public static int GetTwo() => 2;
            public int MethodWithArgs(string arg1, int arg2) { return arg1.Length + arg2; }
            public static int StaticMethodWithArgs(string arg1, int arg2) { return arg1.Length + arg2; }
            public int HostMethod1(string msg)
            {
                return PrintOutput!($"Host Method 1 Received: {msg} from Guest");
            }
            public Func<String, int>? GuestMethod1 = null;
            public Func<String, int>? PrintOutput = null;
        }

        [ExposeToGuest(false)]
        public class ExposeStaticMethodsUsingAttribute
        {
            public void GetNothing() { }
            [ExposeToGuest(true)]
            public static void StaticGetNothing() { }
            [ExposeToGuest(true)]
            public static int StaticGetInt() { return 10; }
            public void GetNothingWithArgs(string arg1, int arg2) { }
            public static void StaticGetNothingWithArgs(string arg1, int arg2) { }
            public static Func<string, int>? GuestMethod = null;
            [ExposeToGuest(true)]
            public static int HostMethod1(string msg)
            {
                return msg.Length;
            }
        }

        [ExposeToGuest(false)]
        public class ExposeInstanceMethodsUsingAttribute
        {
            public void GetNothing() { }
            public static void StaticGetNothing() { }
            public void GetNothingWithArgs(string arg1, int arg2) { }
            public static void StaticGetNothingWithArgs(string arg1, int arg2) { }
            [ExposeToGuest(true)]
            public Func<string, int>? GuestMethod = null;
            [ExposeToGuest(true)]
            public int HostMethod(string msg)
            {
                return PrintOutput!($"Host Received: {msg} from Guest");
            }
            [ExposeToGuest(true)]
            public int HostMethod1(string msg)
            {
                return PrintOutput!($"Host Method 1 Received: {msg} from Guest");
            }
            [ExposeToGuest(true)]
            public Func<String, int>? PrintOutput = null;
        }

        public class DontExposeSomeMembersUsingAttribute
        {
            public int GetOne() => 1;
            public static int GetTwo() => 2;
            public int MethodWithArgs(string arg1, int arg2) { return arg1.Length + arg2; }
            public static int StaticMethodWithArgs(string arg1, int arg2) { return arg1.Length + arg2; }
            [ExposeToGuest(false)]
            public void GetNothing() { }
            [ExposeToGuest(false)]
            public static void StaticGetNothing() { }
            [ExposeToGuest(false)]
            public void GetNothingWithArgs(string arg1, int arg2) { }
            [ExposeToGuest(false)]
            public static void StaticGetNothingWithArgs(string arg1, int arg2) { }
        }

        class GuestFunctionErrors
        {
            public Func<string, int>? FunctionDoesntExist = null;
            public Func<int>? GuestMethod2 = null;
            public Func<int, int>? GuestMethod3 = null;
        }

        public class StackOverflowTests
        {
            public Func<int, int>? StackAllocate;
            public Func<int, int>? StackOverflow;
            public Func<int>? LargeVar;
            public Func<int>? SmallVar;
        }

        public class MultipleGuestFunctionParameters
        {
            public Func<string, int, int>? PrintTwoArgs;
            public Func<string, int, long, int>? PrintThreeArgs;
            public Func<string, int, long, string, int>? PrintFourArgs;
            public Func<string, int, long, string, string, int>? PrintFiveArgs;
            public Func<string, int, long, string, string, bool, int>? PrintSixArgs;
            public Func<string, int, long, string, string, bool, bool, int>? PrintSevenArgs;
            public Func<string, int, long, string, string, bool, bool, uint, int>? PrintEightArgs;
            public Func<string, int, long, string, string, bool, bool, uint, ulong, int>? PrintNineArgs;
            public Func<string, int, long, string, string, bool, bool, uint, ulong, int, int>? PrintTenArgs;
        }

        public class MallocTests
        {
            public Func<int, int>? CallMalloc;
            public Func<int, int>? MallocAndFree;
        }

        public class BufferOverrunTests
        {
            public Func<string, int>? BufferOverrun;
        }

        public class BinaryArrayTests
        {
            public Action<byte[], int>? SetByteArrayToZero;
            public Func<byte[], int>? SetByteArrayToZeroNoLength;
        }

        public class LoggingTests
        {
            public Func<string, string, int, int>? LogMessage;
        }

        [FactSkipIfNotWindowsAndNoHypervisor]
        public void Test_Passing_Byte_Array()
        {

            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            foreach (var option in options)
            {
                var correlationId = Guid.NewGuid().ToString("N");
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
                {
                    var functions = new BinaryArrayTests();
                    sandbox.BindGuestFunction("SetByteArrayToZero", functions);
                    byte[] bytes = new byte[10];
                    RandomNumberGenerator.Create().GetBytes(bytes);
                    var ex = Record.Exception(() =>
                    {
                        functions.SetByteArrayToZero!(bytes, 10);
                    });
                    Assert.Null(ex);
                }

                correlationId = Guid.NewGuid().ToString("N");
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
                {
                    var functions = new BinaryArrayTests();
                    sandbox.BindGuestFunction("SetByteArrayToZeroNoLength", functions);
                    byte[] bytes = new byte[10];
                    RandomNumberGenerator.Create().GetBytes(bytes);
                    var ex = Record.Exception(() =>
                    {
                        _ = functions.SetByteArrayToZeroNoLength!(bytes);
                    });
                    Assert.NotNull(ex);
                    Assert.IsType<System.ArgumentException>(ex);
                    Assert.Equal($"Array length must be specified CorrelationId: {correlationId} Source: SandboxMemoryManager", ex.Message);
                }
            }
        }

        [FactSkipIfNotWindowsAndNoHypervisor]
        public void Test_Heap_Size()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            // Heap size is set on the assembly metadata and linker arguments using the build property GUESTHEAPSIZE
            // this is set in \src\tests\Directory.Build.props
            // the value used can be changed by running msbuild with /p:GUESTHEAPSIZE=VALUE
            var heapSize = GetAssemblyMetadataAttribute("GUESTHEAPSIZE");

            using (var sandbox = new Sandbox(guestBinaryPath, options[0]))
            {
                CheckSize(heapSize, sandbox, "GetHeapSizeAddress");
            }
        }

        public int GetAssemblyMetadataAttribute(string name)
        {
            var value = GetType().Assembly.GetCustomAttributes<AssemblyMetadataAttribute>().Where(a => a.Key == name).Select(a => a.Value).FirstOrDefault();
            Assert.NotNull(value);
            Assert.True(int.TryParse(value, out int intValue));
            return intValue;
        }

        [Fact(Skip = "This test is failing with the cancel vCPU changes.  Need to investigate.")]
        // These tests are proably failing because the test Logger stores logs in ThreadLocal storage
        // See this PR https://github.com/deislabs/hyperlight/pull/1213 which introduced a new logger
        // that works for guest logging.
        public void Test_Error_Logging()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "callbackguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            var message = "This is a test log message";
            var source = "SandboxHostTest";

            foreach (var option in options)
            {
                // If no correlationId or function is provided then the correlationId should change with each invocation.

                foreach (var logLevel in Enum.GetValues(typeof(LogLevel)))
                {
                    var logFunctions = new LoggingTests();
                    var mockLogger = new Mock<ILogger>();
                    mockLogger.Setup(x => x.IsEnabled(It.IsAny<LogLevel>())).Returns(true);
                    var builder = new SandboxBuilder()
                        .WithConfig(GetSandboxConfiguration())
                        .WithRunOptions(option)
                        .WithGuestBinaryPath(guestBinaryPath)
                        .WithErrorMessageLogger(mockLogger.Object);
                    using (var sandbox = builder.Build())
                    {
                        sandbox.BindGuestFunction("LogMessage", logFunctions);
                        logFunctions.LogMessage!(message, source, (int)logLevel);
                        var correlationId = Sandbox.CorrelationId.Value;
                        mockLogger.Verify(
                            l => l.Log<It.IsAnyType>(
                                (LogLevel)logLevel,
                                It.IsAny<EventId>(),
                                It.Is<It.IsAnyType>((v, t) => v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationId} Source: {source}")),
                                null,
                                It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                                Times.Once);
                        if (option.HasFlag(SandboxRunOptions.RecycleAfterRun))
                        {
                            logFunctions.LogMessage!(message, source, (int)logLevel);
                            correlationId = Sandbox.CorrelationId.Value;
                            mockLogger.Verify(
                                l => l.Log<It.IsAnyType>(
                                    (LogLevel)logLevel,
                                    It.IsAny<EventId>(),
                                    It.Is<It.IsAnyType>((v, t) => v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationId} Source: {source}")),
                                    null,
                                    It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                                    Times.Once);
                        }
                    }
                }

                // If a correlationId provided then the correlationId should always be used;

                foreach (var logLevel in Enum.GetValues(typeof(LogLevel)))
                {
                    var logFunctions = new LoggingTests();
                    var mockLogger = new Mock<ILogger>();
                    mockLogger.Setup(x => x.IsEnabled(It.IsAny<LogLevel>())).Returns(true);
                    var correlationId = Guid.NewGuid().ToString("N");
                    var builder = new SandboxBuilder()
                        .WithConfig(GetSandboxConfiguration())
                        .WithRunOptions(option)
                        .WithGuestBinaryPath(guestBinaryPath)
                        .WithCorrelationId(correlationId)
                        .WithErrorMessageLogger(mockLogger.Object);
                    using (var sandbox = builder.Build())
                    {
                        sandbox.BindGuestFunction("LogMessage", logFunctions);
                        logFunctions.LogMessage!(message, source, (int)logLevel);
                        mockLogger.Verify(
                            l => l.Log<It.IsAnyType>(
                                (LogLevel)logLevel,
                                It.IsAny<EventId>(),
                                It.Is<It.IsAnyType>((v, t) => v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationId} Source: {source}")),
                                null,
                                It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                                Times.Once);
                        if (option.HasFlag(SandboxRunOptions.RecycleAfterRun))
                        {
                            logFunctions.LogMessage!(message, source, (int)logLevel);
                            mockLogger.Verify(
                                l => l.Log<It.IsAnyType>(
                                    (LogLevel)logLevel,
                                    It.IsAny<EventId>(),
                                    It.Is<It.IsAnyType>((v, t) => v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationId} Source: {source}")),
                                    null,
                                    It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                                    Times.Exactly(2));
                        }
                    }
                }

                string correlationIdFromLocalFunc = string.Empty;
                var callCount = 0;
                var calledCount = 0;
                string GetCorrelationId()
                {
                    calledCount++;
                    correlationIdFromLocalFunc = Guid.NewGuid().ToString("N");
                    return correlationIdFromLocalFunc;
                }

                // If a function is provided then the function should be called for each invocation.

                foreach (var logLevel in Enum.GetValues(typeof(LogLevel)))
                {
                    var logFunctions = new LoggingTests();
                    var mockLogger = new Mock<ILogger>();
                    mockLogger.Setup(x => x.IsEnabled(It.IsAny<LogLevel>())).Returns(true);
                    var builder = new SandboxBuilder()
                        .WithConfig(GetSandboxConfiguration())
                        .WithRunOptions(option)
                        .WithGuestBinaryPath(guestBinaryPath)
                        .WithCorrelationId(GetCorrelationId)
                        .WithErrorMessageLogger(mockLogger.Object);
                    using (var sandbox = builder.Build())
                    {
                        callCount++;
                        Assert.Equal(callCount, calledCount);
                        sandbox.BindGuestFunction("LogMessage", logFunctions);
                        logFunctions.LogMessage!(message, source, (int)logLevel);
                        callCount++;
                        Assert.Equal(callCount, calledCount);
                        mockLogger.Verify(
                            l => l.Log<It.IsAnyType>(
                                (LogLevel)logLevel,
                                It.IsAny<EventId>(),
                                It.Is<It.IsAnyType>((v, t) => v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationIdFromLocalFunc} Source: {source}")),
                                null,
                                It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                                Times.Once);
                        if (option.HasFlag(SandboxRunOptions.RecycleAfterRun))
                        {

                            logFunctions.LogMessage!(message, source, (int)logLevel);
                            callCount++;
                            Assert.Equal(callCount, calledCount);
                            mockLogger.Verify(
                                l => l.Log<It.IsAnyType>(
                                    (LogLevel)logLevel,
                                    It.IsAny<EventId>(),
                                    It.Is<It.IsAnyType>((v, t) => v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationIdFromLocalFunc} Source: {source}")),
                                    null,
                                    It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                                    Times.Once);
                        }
                    }
                }
            }
        }

        [Fact(Skip = "This test is failing with the cancel vCPU changes.  Need to investigate.")]
        public void Test_Error_Logging_Cross_ExecutionContext()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "callbackguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            var message = "This is a test log message";
            var source = "SandboxHostTest";
            foreach (var option in options)
            {
                // If no correlationId or function is provided then a new one is generated for each invocation.

                foreach (var logLevel in Enum.GetValues(typeof(LogLevel)))
                {
                    var correlationId1 = string.Empty;
                    var correlationId2 = string.Empty;
                    var logFunctions = new LoggingTests();
                    var mockLogger = new Mock<ILogger>();
                    mockLogger.Setup(x => x.IsEnabled(It.IsAny<LogLevel>())).Returns(true);
                    var builder = new SandboxBuilder()
                        .WithRunOptions(option)
                        .WithGuestBinaryPath(guestBinaryPath)
                        .WithErrorMessageLogger(mockLogger.Object);
                    using (var sandbox = builder.Build())
                    {
                        sandbox.BindGuestFunction("LogMessage", logFunctions);
                        Exception? threadTestException = null;

                        // Make sure that CorrelationId is correct after a call on a different thread with a different execution context from the one that constructed the Sandbox
                        void threadStart1()
                        {
                            threadTestException = Record.Exception(() =>
                            {
                                logFunctions.LogMessage!(message, source, (int)logLevel);
                                correlationId1 = Sandbox.CorrelationId.Value;
                            });
                        }

                        var thread = new Thread(threadStart1);

                        // UnsafeStart causes the thread to start without flowing the ExecutionContext from the current thread.

                        thread.UnsafeStart();
                        thread.Join();
                        Assert.Null(threadTestException);
                        mockLogger.Verify(
                        l => l.Log<It.IsAnyType>(
                            (LogLevel)logLevel,
                            It.IsAny<EventId>(),
                            It.Is<It.IsAnyType>((v, t) =>
                                v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationId1} Source: {source}")
                            ),
                            null,
                            It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                            Times.Once);

                        if (option.HasFlag(SandboxRunOptions.RecycleAfterRun))
                        {
                            void threadStart2()
                            {
                                threadTestException = Record.Exception(() =>
                                {
                                    logFunctions.LogMessage!(message, source, (int)logLevel);
                                    correlationId2 = Sandbox.CorrelationId.Value;
                                });
                            }
                            thread = new Thread(threadStart2);
                            thread.UnsafeStart();
                            thread.Join();
                            Assert.Null(threadTestException);
                            mockLogger.Verify(
                                l => l.Log<It.IsAnyType>(
                                    (LogLevel)logLevel,
                                    It.IsAny<EventId>(),
                                    It.Is<It.IsAnyType>((v, t) => v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationId2} Source: {source}")),
                                    null,
                                    It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                                    Times.Once);
                        }
                    }
                }

                // If a correlationId is provided in the constructor then the value should always be used for the correlationId for the life of the Sandbox.
                foreach (var logLevel in Enum.GetValues(typeof(LogLevel)))
                {
                    var correlationId = Guid.NewGuid().ToString("N");
                    var logFunctions = new LoggingTests();
                    var mockLogger = new Mock<ILogger>();
                    mockLogger.Setup(x => x.IsEnabled(It.IsAny<LogLevel>())).Returns(true);
                    var builder = new SandboxBuilder()
                        .WithCorrelationId(correlationId)
                        .WithRunOptions(option)
                        .WithGuestBinaryPath(guestBinaryPath)
                        .WithErrorMessageLogger(mockLogger.Object);
                    using (var sandbox = builder.Build())
                    {
                        sandbox.BindGuestFunction("LogMessage", logFunctions);
                        Exception? threadTestException = null;

                        // Make sure that CorrelationId is correct after a call on a different thread with a different execution context from the one that constructed the Sandbox
                        void threadStart()
                        {
                            threadTestException = Record.Exception(() =>
                            {
                                logFunctions.LogMessage!(message, source, (int)logLevel);
                            });
                        }

                        var thread = new Thread(threadStart);

                        // UnsafeStart causes the thread to start without flowing the ExecutionContext from the current thread.

                        thread.UnsafeStart();
                        thread.Join();
                        Assert.Null(threadTestException);
                        mockLogger.Verify(
                           l => l.Log<It.IsAnyType>(
                               (LogLevel)logLevel,
                               It.IsAny<EventId>(),
                               It.Is<It.IsAnyType>((v, t) =>
                                   v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationId} Source: {source}")
                               ),
                               null,
                               It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                               Times.Once);

                        if (option.HasFlag(SandboxRunOptions.RecycleAfterRun))
                        {
                            thread = new Thread(threadStart);
                            thread.UnsafeStart();
                            thread.Join();
                            Assert.Null(threadTestException);
                            mockLogger.Verify(
                                l => l.Log<It.IsAnyType>(
                                    (LogLevel)logLevel,
                                    It.IsAny<EventId>(),
                                    It.Is<It.IsAnyType>((v, t) => v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationId} Source: {source}")),
                                    null,
                                    It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                                    Times.Exactly(2));
                        }
                    }
                }
                string correlationIdFromLocalFunc = string.Empty;
                var callCount = 0;
                var calledCount = 0;
                string GetCorrelationId()
                {
                    calledCount++;
                    correlationIdFromLocalFunc = Guid.NewGuid().ToString("N");
                    return correlationIdFromLocalFunc;
                }
                // If a function is provided then the function should be called during construction and once for each invocation.
                foreach (var logLevel in Enum.GetValues(typeof(LogLevel)))
                {
                    var logFunctions = new LoggingTests();
                    var mockLogger = new Mock<ILogger>();
                    mockLogger.Setup(x => x.IsEnabled(It.IsAny<LogLevel>())).Returns(true);
                    var builder = new SandboxBuilder()
                        .WithRunOptions(option)
                        .WithGuestBinaryPath(guestBinaryPath)
                        .WithCorrelationId(GetCorrelationId)
                        .WithErrorMessageLogger(mockLogger.Object);
                    using (var sandbox = builder.Build())
                    {
                        // function should get called from the constructor.
                        callCount++;
                        Assert.Equal(callCount, calledCount);
                        sandbox.BindGuestFunction("LogMessage", logFunctions);

                        Exception? threadTestException = null;

                        // Make sure that CorrelationId is correct after a recycle on a different thread.
                        void threadStart()
                        {
                            threadTestException = Record.Exception(() =>
                            {
                                logFunctions.LogMessage!(message, source, (int)logLevel);
                                callCount++;
                            });
                        }

                        var thread = new Thread(threadStart);


                        // UnsafeStart causes the thread to start without flowing the ExecutionContext from the current thread.

                        thread.UnsafeStart();
                        thread.Join();
                        Assert.Null(threadTestException);
                        Assert.Equal(callCount, calledCount);
                        mockLogger.Verify(
                        l => l.Log<It.IsAnyType>(
                            (LogLevel)logLevel,
                            It.IsAny<EventId>(),
                            It.Is<It.IsAnyType>((v, t) =>
                                v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationIdFromLocalFunc} Source: {source}")
                            ),
                            null,
                            It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                            Times.Once);

                        if (option.HasFlag(SandboxRunOptions.RecycleAfterRun))
                        {
                            thread = new Thread(threadStart);

                            // UnsafeStart causes the thread to start without flowing the ExecutionContext from the current thread.

                            thread.UnsafeStart();
                            thread.Join();
                            Assert.Null(threadTestException);
                            Assert.Equal(callCount, calledCount);
                            mockLogger.Verify(
                               l => l.Log<It.IsAnyType>(
                                   (LogLevel)logLevel,
                                   It.IsAny<EventId>(),
                                   It.Is<It.IsAnyType>((v, t) =>
                                       v.ToString()!.StartsWith($"ErrorMessage: {message} CorrelationId: {correlationIdFromLocalFunc} Source: {source}")
                                   ),
                                   null,
                                   It.IsAny<Func<It.IsAnyType?, Exception?, string>>()),
                                   Times.Once);
                        }
                    }
                }
            }
        }

        [Fact(Skip = "This test is failing with the cancel vCPU changes.  Need to investigate.")]
        public void Test_Handles_Host_Exception()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "callbackguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            var message = "This is a test exception message";
            foreach (var option in options)
            {
                foreach (var logLevel in Enum.GetValues(typeof(LogLevel)))
                {
                    var hostExceptionTest = new HostExceptionTestMembers();

                    var correlationId = Guid.NewGuid().ToString("N");
                    var builder = new SandboxBuilder()
                        .WithRunOptions(option)
                        .WithGuestBinaryPath(guestBinaryPath)
                        .WithCorrelationId(correlationId);
                    using (var sandbox = builder.Build())
                    {
                        sandbox.ExposeAndBindMembers(hostExceptionTest);
                        var ex = Record.Exception(() =>
                        {
                            hostExceptionTest.CallErrorMethod!(message);
                        });
                        Assert.NotNull(ex);
                        Assert.IsType<Hyperlight.Core.HyperlightException>(ex);
                        Assert.Equal($"Error From Host: {message} CorrelationId: {correlationId} Source: HostExceptionTestMembers", ex.Message);
                    }
                }
            }
        }

        [FactSkipIfNotWindowsAndNoHypervisor]
        public void Test_Stack_Size()
        {
            using var ctx = new Wrapper.Context("sample_corr_id");
            var options = GetSandboxRunOptions();
            if (options.Length == 0)
            {
                return;
            }

            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            // Stack size is set on the assembly metadata and linker arguments using the build property GUESTSTACKSIZE
            // this is set in \src\tests\Directory.Build.props
            // the value used can be changed by running msbuild with /p:GUESTSTACKSIZE=VALUE
            var stackSize = GetAssemblyMetadataAttribute("GUESTSTACKSIZE");

            using (var sandbox = new Sandbox(guestBinaryPath, options[0]))
            {
                var bindingFlags = BindingFlags.NonPublic | BindingFlags.Instance;
                var fieldInfo = sandbox.GetType().GetField("sandboxMemoryManager", bindingFlags);
                Assert.NotNull(fieldInfo);
                var sandboxMemoryManager = fieldInfo!.GetValue(sandbox);
                Assert.NotNull(sandboxMemoryManager);
                var layoutPropertyInfo = sandboxMemoryManager!.GetType().GetProperty("sandboxMemoryLayout", bindingFlags);
                Assert.NotNull(fieldInfo);
            }
        }

        [FactSkipIfNotWindows]
        public void Test_Memory_Size_From_GuestBinary()
        {
            using var ctx = new Wrapper.Context("sample_corr_id");
            var option = SandboxRunOptions.RunFromGuestBinary;
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            var sandboxConfiguration = new SandboxConfiguration();

            ulong expectedSize = GetExpectedMemorySize(sandboxConfiguration, guestBinaryPath, option);

            using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, null, null, sandboxConfiguration))
            {
                var size = GetMemorySize(sandbox);
                Assert.Equal(expectedSize, size);
            }

        }

        [FactSkipIfNotWindows]
        public void Test_Memory_Size_InProcess()
        {
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            var sandboxConfiguration = new SandboxConfiguration();
            var options = new SandboxRunOptions[] { SandboxRunOptions.RunInProcess, SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun };

            foreach (var option in options)
            {
                ulong expectedSize = GetExpectedMemorySize(sandboxConfiguration, guestBinaryPath, option);
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, null, null, sandboxConfiguration))
                {
                    var size = GetMemorySize(sandbox);
                    Assert.Equal(expectedSize, size);
                }
            }
        }

        [Fact]
        public void Test_Buffer_Overrun()
        {
            using var ctx = new Wrapper.Context("sample_corr_id");
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            var options = GetSandboxRunOptions();
            foreach (var option in options)
            {
                var correlationId = Guid.NewGuid().ToString("N");
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
                {
                    var functions = new BufferOverrunTests();
                    sandbox.BindGuestFunction("BufferOverrun", functions);
                    var ex = Record.Exception(() =>
                    {
                        string arg = "This is a test and it should cause a GS_CHECK_FAILED error";
                        functions.BufferOverrun!(arg);
                    });
                    //Assert.NotNull(ex);
                    //Assert.IsType<Hyperlight.Core.HyperlightException>(ex);
                    //Assert.Equal($"GsCheckFailed: CorrelationId: {correlationId} Source: Sandbox", ex.Message);
                    // ^^^ TODO(#1057): I am commenting out these lines
                    // because we currently don't have
                    // stack guards in place.
                }
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, null, null, GetSandboxConfiguration()))
                {
                    var functions = new BufferOverrunTests();
                    sandbox.BindGuestFunction("BufferOverrun", functions);
                    var result = 0;
                    var ex = Record.Exception(() =>
                    {
                        string arg = "This should work!";
                        result = functions.BufferOverrun!(arg);
                    });
                    Assert.Null(ex);

                    Assert.Equal(0, result);
                }
            }
        }

        // The following tests fails if the shared input/output buffer
        // can be overwritten. If the input/output buffers are properly
        // implemented as a stack, this test should pass.
        [FactSkipIfNotCGuest]
        public void Test_InputOutput_Stack()
        {
            using var ctx = new Wrapper.Context("sample_corr_id");
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            var options = GetSandboxRunInHyperVisorOptions();
            foreach (var option in options)
            {
                var correlationId = Guid.NewGuid().ToString("N");
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, null, null, GetSandboxConfiguration()))
                {
                    var functions = new DontExistMembers();
                    sandbox.BindGuestFunction("ThisIsNotARealFunctionButTheNameIsImportant", functions);
                    var result = 0;
                    var ex = Record.Exception(() =>
                    {
                        result = functions.ThisIsNotARealFunctionButTheNameIsImportant!();
                    });
                    // should not fail
                    Assert.Null(ex);

                    Assert.Equal(99, result);
                    // 99 is a special return value to indicate good execution,
                    // see c simpleguest dynamic GuestDispatchFunction fn
                }
                correlationId = Guid.NewGuid().ToString("N");
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, null, null, GetSandboxConfiguration()))
                {
                    var functions = new DontExistMembers();
                    sandbox.BindGuestFunction("SomethingElse", functions);
                    var result = 0;
                    var ex = Record.Exception(() =>
                    {
                        result = functions.SomethingElse!();
                    });
                    // should fail
                    Assert.NotNull(ex);
                }
            }
        }

        [FactSkipIfHypervisorNotPresent]
        public void Test_Stack_Overflow()
        {
            // These are commented out because there currently exists no easy way to get back into dotnet code from an
            // outb abort. Therefore these tests are replicated in rust hyperlight_host integration tests.
            // TODO We should re-enable these tests once we have a way to get the exit code from the outb handler.
            // see issue https://github.com/deislabs/hyperlight/issues/1184.

            // var guestBinaryFileName = "simpleguest.exe";
            // var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            // var options = GetSandboxRunInHyperVisorOptions();
            // var size = GetAssemblyMetadataAttribute("GUESTSTACKSIZE") / 2;

            // // StackOverflow(int) function allocates a 16384 sized array and recursively calls itself int times

            // var shouldNotOverflow = (GetAssemblyMetadataAttribute("GUESTSTACKSIZE") / 16384) - 2;
            // var shouldOverflow = (GetAssemblyMetadataAttribute("GUESTSTACKSIZE") / 16384) + 1;

            // foreach (var option in options)
            // {
            //     var correlationId = Guid.NewGuid().ToString("N");
            //     using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
            //     {
            //         var functions = new StackOverflowTests();
            //         sandbox.BindGuestFunction("StackAllocate", functions);
            //         var ex = Record.Exception(() =>
            //         {
            //             int arg = 0;
            //             functions.StackAllocate!(arg);
            //         });
            //         Assert.NotNull(ex);
            //         Assert.IsType<System.StackOverflowException>(ex);
            //         Assert.Equal($"Guest Error CorrelationId: {correlationId} Source: Sandbox", ex.Message);

            //     }
            //     correlationId = Guid.NewGuid().ToString("N");
            //     using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
            //     {
            //         var functions = new StackOverflowTests();
            //         sandbox.BindGuestFunction("StackAllocate", functions);
            //         var result = functions.StackAllocate!(size);
            //         Assert.Equal(size, result);
            //     }
            //     correlationId = Guid.NewGuid().ToString("N");
            //     using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
            //     {
            //         var functions = new StackOverflowTests();
            //         sandbox.BindGuestFunction("StackOverflow", functions);
            //         var ex = Record.Exception(() =>
            //         {
            //             var result = functions.StackOverflow!(shouldOverflow);
            //         });
            //         Assert.NotNull(ex);
            //         Assert.IsType<System.StackOverflowException>(ex);
            //         Assert.Equal($"Guest Error CorrelationId: {correlationId} Source: Sandbox", ex.Message);
            //     }
            //     correlationId = Guid.NewGuid().ToString("N");
            //     using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
            //     {
            //         var memorySize = GetMemorySize(sandbox);
            //         var iterations = (int)(memorySize / 16384) * 2;
            //         var functions = new StackOverflowTests();
            //         sandbox.BindGuestFunction("StackOverflow", functions);
            //         var ex = Record.Exception(() =>
            //         {
            //             var result = functions.StackOverflow!(iterations);
            //         });
            //         Assert.NotNull(ex);
            //         Assert.IsType<System.StackOverflowException>(ex);
            //         Assert.Equal($"Guest Error CorrelationId: {correlationId} Source: Sandbox", ex.Message);
            //     }

            //     correlationId = Guid.NewGuid().ToString("N");
            //     using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
            //     {
            //         var functions = new StackOverflowTests();
            //         sandbox.BindGuestFunction("LargeVar", functions);
            //         var ex = Record.Exception(() =>
            //         {
            //             var result = functions.LargeVar!();
            //         });
            //         Assert.NotNull(ex);
            //         Assert.IsType<System.StackOverflowException>(ex);
            //         Assert.Equal($"Guest Error CorrelationId: {correlationId} Source: Sandbox", ex.Message);
            //     }

            //     correlationId = Guid.NewGuid().ToString("N");
            //     using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
            //     {
            //         var functions = new StackOverflowTests();
            //         sandbox.BindGuestFunction("SmallVar", functions);
            //         var result = functions.SmallVar!();
            //         Assert.Equal(1024, result);
            //     }
            // }
        }

        [FactSkipIfHypervisorNotPresent]
        public void Test_Memory_Size_InHypervisor()
        {
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            var sandboxConfiguration = new SandboxConfiguration();
            var options = new SandboxRunOptions[] { SandboxRunOptions.None, SandboxRunOptions.RecycleAfterRun, SandboxRunOptions.None | SandboxRunOptions.RecycleAfterRun };

            foreach (var option in options)
            {
                ulong expectedSize = GetExpectedMemorySize(sandboxConfiguration, guestBinaryPath, option);
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, null, null, sandboxConfiguration))
                {
                    var size = GetMemorySize(sandbox);
                    Assert.Equal(expectedSize, size);
                }
            }
        }

        [FactSkipIfNotWindows]
        public void Test_Maximum_Memory_Size()
        {
            var correlationId = Guid.NewGuid().ToString("N");
            using var ctx = new Wrapper.Context(correlationId);
            var option = SandboxRunOptions.RunInProcess;
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            var sandboxConfiguration = new SandboxConfiguration()
                .WithInputDataSize(1073741824);
            var ex = Record.Exception(() =>
            {
                using var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, sandboxConfiguration);

            });
            Assert.NotNull(ex);
            Assert.IsType<HyperlightException>(ex);
            string expectedPattern = $"Memory requested [0-9]+ exceeds maximum size allowed 1071644672 CorrelationId: {correlationId} Source: NativeHandleWrapperErrorExtensions";
            Assert.Matches(expectedPattern, ex.Message);
        }

        private ulong GetExpectedMemorySize(SandboxConfiguration sandboxConfiguration, string guestBinaryPath, SandboxRunOptions option)
        {

            // Heap size and Stack size are set on the assembly metadata and linker arguments using the build property GUESTHEAPSIZE and GUESTSTACKSIZE
            // this is set in \src\tests\Directory.Build.props
            // the value used can be changed by running msbuild with /p:GUESTHEAPSIZE=VALUE
            var heapSize = (ulong)GetAssemblyMetadataAttribute("GUESTHEAPSIZE");

            // the value used can be changed by running msbuild with /p:GUESTSTACKSIZE=VALUE
            var stackSize = (ulong)GetAssemblyMetadataAttribute("GUESTSTACKSIZE");

            ulong pageTableSize = 0x3000;
            var headerSize = (ulong)120;
            ulong totalSize;
            var codeSize = (ulong)0;
            if (!option.HasFlag(SandboxRunOptions.RunFromGuestBinary))
            {
                var codePayload = File.ReadAllBytes(guestBinaryPath);
                codeSize = (ulong)codePayload.Length;
            }
            totalSize = (ulong)(pageTableSize
                                + codeSize
                                + round_up(headerSize)
                                + sandboxConfiguration.HostFunctionDefinitionSize
                                + round_up(sandboxConfiguration.HostExceptionSize
                                + sandboxConfiguration.GuestErrorBufferSize
                                + sandboxConfiguration.InputDataSize
                                + sandboxConfiguration.OutputDataSize
                                + sandboxConfiguration.GuestPanicBufferSize
                                + heapSize)
                                + PAGE_SIZE // stack guard page
                                + round_up(stackSize)
                                + 2 * PAGE_SIZE); // surrounding guard pages


            return totalSize;
        }

        private ulong round_up(ulong size)
        {
            return (size + 4096ul - 1) & ~(4096ul - 1);
        }

        private static ulong GetMemorySize(Sandbox sandbox)
        {
            return sandbox.memSize;
        }

        [Fact]
        public void Test_Guest_Error_Message_Size()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);

            foreach (var option in options)
            {
                var size = GetErrorMessageSize();
                var bld = new SandboxBuilder()
                    .WithGuestBinaryPath(guestBinaryPath)
                    .WithRunOptions(option)
                    .WithConfig(
                        new SandboxConfiguration()
                            .WithGuestErrorBufferSize((ulong)size)
                    );
                using (var sandbox = bld.Build())
                {
                    CheckSize((int)size, sandbox, "GetGuestErrorBufferSizeAddress");
                }
            }
        }

        [Fact]
        public void Test_Function_Definitions_Size()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);

            foreach (var option in options)
            {
                var size = GetFunctionDefinitionSize();
                var bld = new SandboxBuilder()
                    .WithGuestBinaryPath(guestBinaryPath)
                    .WithRunOptions(option)
                    .WithConfig(
                        new SandboxConfiguration()
                            .WithHostFunctionDefinitionSize((ulong)size)
                    );
                using (var sandbox = bld.Build())
                {
                    CheckSize((int)size, sandbox, "GetFunctionDefinitionSizeAddress");
                }
            }
        }

        [Fact]
        public void Test_InputData_Size()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);

            foreach (var option in options)
            {
                var size = GetInputDataSize();
                var bld = new SandboxBuilder()
                    .WithGuestBinaryPath(guestBinaryPath)
                    .WithRunOptions(option)
                    .WithConfig(
                        new SandboxConfiguration()
                            .WithInputDataSize((ulong)size)
                    );
                using (var sandbox = bld.Build())
                {
                    CheckSize((int)size, sandbox, "GetInputDataSizeAddress");
                }
            }
        }

        [Fact]
        public void Test_OutputData_Size()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);

            foreach (var option in options)
            {
                var size = GetOutputDataSize();
                var bld = new SandboxBuilder()
                    .WithGuestBinaryPath(guestBinaryPath)
                    .WithRunOptions(option)
                    .WithConfig(
                        new SandboxConfiguration()
                            .WithOutputDataSize((ulong)size)
                    );
                using (var sandbox = bld.Build())
                {
                    CheckSize((int)size, sandbox, "GetOutputDataSizeAddress");
                }
            }
        }

        [Fact]
        public void Test_Config_Minimum_Sizes()
        {
            ulong minInputSize = 0x2000;
            ulong minOutputSize = 0x2000;
            ulong minHostFunctionDefinitionSize = 0x1000;
            ulong minHostExceptionSize = 0x4000;
            ulong minGuestErrorBufferSize = 0x80;
            ushort defaultMaxExecutionTime = 1000;
            ushort defaultMaxWaitForCancellation = 100;
            var sandboxConfiguration = new SandboxConfiguration(0, 0, 0, 0, 0, 0, 0);
            Assert.Equal(minInputSize, sandboxConfiguration.InputDataSize);
            Assert.Equal(minOutputSize, sandboxConfiguration.OutputDataSize);
            Assert.Equal(minHostFunctionDefinitionSize, sandboxConfiguration.HostFunctionDefinitionSize);
            Assert.Equal(minHostExceptionSize, sandboxConfiguration.HostExceptionSize);
            Assert.Equal(minGuestErrorBufferSize, sandboxConfiguration.GuestErrorBufferSize);
            Assert.Equal(defaultMaxExecutionTime, sandboxConfiguration.MaxExecutionTime);
            Assert.Equal(defaultMaxWaitForCancellation, sandboxConfiguration.MaxWaitForCancellation);
        }

        [Fact]
        public void Test_Host_Exceptions_Size()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);

            foreach (var option in options)
            {
                var size = GetHostExceptionSize();
                var bld = new SandboxBuilder()
                .WithGuestBinaryPath(guestBinaryPath)
                .WithRunOptions(option)
                .WithConfig(
                    new SandboxConfiguration()
                        .WithHostExceptionSize((ulong)size)
                );
                using (var sandbox = bld.Build())
                {
                    CheckSize((int)size, sandbox, "GetHostExceptionSizeAddress");
                }
            }
        }

        private static void CheckSize(int size, Sandbox sandbox, string methodName)
        {
            var bindingFlags = BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Public;
            var fieldInfo = sandbox.GetType().GetField("sandboxMemoryManager", bindingFlags);
            Assert.NotNull(fieldInfo);
            var sandboxMemoryManager = fieldInfo!.GetValue(sandbox);
            Assert.NotNull(sandboxMemoryManager);
            Assert.NotNull(fieldInfo);
            var propInfo = sandboxMemoryManager!.GetType().GetProperty("SourceAddress", bindingFlags);
            Assert.NotNull(propInfo);
            var sourceAddress = propInfo!.GetValue(sandboxMemoryManager);
            Assert.NotNull(sourceAddress);
        }

        [Fact]
        public void Test_Invalid_Guest_Function_Causes_Exception()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "callbackguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);

            foreach (var option in options)
            {
                var correlationId = Guid.NewGuid().ToString("N");
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
                {
                    var functions = new GuestFunctionErrors();
                    sandbox.BindGuestFunction("FunctionDoesntExist", functions);
                    var ex = Record.Exception(() =>
                    {
                        string arg = string.Empty;
                        functions.FunctionDoesntExist!(arg);
                    });
                    Assert.NotNull(ex);
                    Assert.IsType<HyperlightException>(ex);
                    Assert.Equal($"GuestFunctionNotFound:FunctionDoesntExist CorrelationId: {correlationId} Source: Sandbox", ex.Message);
                }
            }
        }

        [Fact]
        public void Test_Multiple_Guest_Function_Parameters()
        {
            using var ctx = new Wrapper.Context("sample_corr_id");
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);

            List<(string Method, object[] args, int returnValue, string expectedOutput)> testData = new()
            {
                (
                    "PrintTwoArgs",
                    new object[] { "Test2", 1 },
                    27,
                    "Message: arg1:Test2 arg2:1."
                ),
                (
                    "PrintThreeArgs",
                    new object[] { "Test3", 2, 3 },
                    34,
                    "Message: arg1:Test3 arg2:2 arg3:3."
                ),
                (
                    "PrintFourArgs",
                    new object[] { "Test4", 3, 4, "Tested" },
                    46,
                    "Message: arg1:Test4 arg2:3 arg3:4 arg4:Tested."
                ),
                (
                    "PrintFiveArgs",
                    new object[] { "Test5", 5, 6, "Tested", "Test5" },
                    57,
                    "Message: arg1:Test5 arg2:5 arg3:6 arg4:Tested arg5:Test5."
                ),
                (
                    "PrintSixArgs",
                    new object[] { "Test6", 7, 8, "Tested", "Test6", false },
                    68,
                    "Message: arg1:Test6 arg2:7 arg3:8 arg4:Tested arg5:Test6 arg6:false."
                ),
                (
                    "PrintSevenArgs",
                    new object[] { "Test7", 8, 9, "Tested", "Test7", false, true },
                    78,
                    "Message: arg1:Test7 arg2:8 arg3:9 arg4:Tested arg5:Test7 arg6:false arg7:true."
                ),
                (
                    "PrintEightArgs",
                    new object[] { "Test8", 10, 11, "Tested", "Test8", false, true, (uint)12 },
                    88,
                    "Message: arg1:Test8 arg2:10 arg3:11 arg4:Tested arg5:Test8 arg6:false arg7:true arg8:12."
                ),
                (
                    "PrintNineArgs",
                    new object[] { "Test9", 12, 13, "Tested", "Test9", true, false, (uint)14, (ulong)15 },
                    96,
                    "Message: arg1:Test9 arg2:12 arg3:13 arg4:Tested arg5:Test9 arg6:true arg7:false arg8:14 arg9:15."
                ),
                (
                    "PrintTenArgs",
                    new object[] { "Test10", 15, 16, "Tested", "Test10", true, false, (uint)17, (ulong)18, 19 },
                    107,
                    "Message: arg1:Test10 arg2:15 arg3:16 arg4:Tested arg5:Test10 arg6:true arg7:false arg8:17 arg9:18 arg10:19."
                )
            };

            foreach (var (method, args, returnValue, expectedOutput) in testData)
            {
                foreach (var option in options)
                {
                    int notused = 0;
                    var testoutput = new StringWriter();
                    var sboxBuilder = new SandboxBuilder()
                        .WithConfig(GetSandboxConfiguration())
                        .WithGuestBinaryPath(guestBinaryPath)
                        .WithRunOptions(option)
                        .WithWriter(testoutput);
                    using (var sandbox = sboxBuilder.Build())
                    {
                        var functions = new MultipleGuestFunctionParameters();
                        sandbox.BindGuestFunction(method, functions);
                        InvokeMethod(functions, method, returnValue, expectedOutput, args, testoutput, ref notused);
                    }
                }
            }
        }

        [Fact]
        public void Test_Invalid_Type_Of_Guest_Function_Parameter_Causes_Exception()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "callbackguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);

            foreach (var option in options)
            {
                var correlationId = Guid.NewGuid().ToString("N");
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
                {
                    var functions = new GuestFunctionErrors();
                    sandbox.BindGuestFunction("GuestMethod3", functions);
                    var ex = Record.Exception(() =>
                    {
                        functions.GuestMethod3!(1);
                    });
                    Assert.NotNull(ex);
                    Assert.IsType<HyperlightException>(ex);
                    Assert.Equal($"GuestFunctionParameterTypeMismatch:Function GuestMethod3 parameter 0. CorrelationId: {correlationId} Source: Sandbox", ex.Message);
                }
            }
        }

        [Fact]
        public void Test_Invalid_Number_Of_Guest_Function_Parameters_Causes_Exception()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "callbackguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);

            foreach (var option in options)
            {
                var correlationId = Guid.NewGuid().ToString("N");
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, correlationId, null, GetSandboxConfiguration()))
                {
                    var functions = new GuestFunctionErrors();
                    sandbox.BindGuestFunction("GuestMethod2", functions);
                    var ex = Record.Exception(() =>
                    {
                        functions.GuestMethod2!();
                    });
                    Assert.NotNull(ex);
                    Assert.IsType<HyperlightException>(ex);
                    Assert.Equal($"GuestFunctionIncorrecNoOfParameters:Called function GuestMethod2 with 0 parameters but it takes 1. CorrelationId: {correlationId} Source: Sandbox", ex.Message);
                }
            }
        }

        [Fact()]
        public void Test_Guest_Malloc()
        {
            using var ctx = new Wrapper.Context("sample_corr_id");
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            var heapSize = GetAssemblyMetadataAttribute("GUESTHEAPSIZE");
            // dlmalloc minimum allocation is 64K
            // simpleguest.exe will do a memory allocation prior to CallMalloc being called;
            // however it will only use a small amount 
            // Therefore an alloction of half the heapsize should always succeed if the heap is at least 128K
            // And heapsize + 64K should always fail.
            Assert.True(heapSize >= 131072);
            foreach (var option in options)
            {
                var logFunctions = new LoggingTests();
                var mockLogger = new Mock<ILogger>();
                mockLogger.Setup(x => x.IsEnabled(It.IsAny<LogLevel>())).Returns(true);
                var correlationId = Guid.NewGuid().ToString("N");
                var builder = new SandboxBuilder()
                    .WithCorrelationId(correlationId)
                    .WithConfig(GetSandboxConfiguration())
                    .WithRunOptions(option)
                    .WithGuestBinaryPath(guestBinaryPath)
                    .WithErrorMessageLogger(mockLogger.Object);
                using (var sandbox = builder.Build())
                {
                    // var functions = new MallocTests();
                    // sandbox.BindGuestFunction("CallMalloc", functions);
                    // testOutput.WriteLine($"Testing CallMalloc with GuestHeapSize:{heapSize} MallocSize:{mallocSize} option: {option}");
                    // var ex = Record.Exception(() =>
                    // {
                    //     functions.CallMalloc!(mallocSize);
                    // });
                    //Assert.NotNull(ex);
                    //Assert.IsType<HyperlightException>(ex);
                    //Assert.StartsWith(
                    //    "Guest Aborted",
                    //    ex.Message
                    //);
                    // When running in process, if rust guest is OOM, it will panic and cause a crash of the host process.
                }
            }
            foreach (var option in options)
            {
                var mallocSize = heapSize / 2;
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, null, null, GetSandboxConfiguration()))
                {
                    var functions = new MallocTests();

                    testOutput.WriteLine($"Testing CallMalloc with GuestHeapSize:{heapSize} MallocSize:{mallocSize} option: {option}");
                    sandbox.BindGuestFunction("CallMalloc", functions);
                    var result = functions.CallMalloc!(mallocSize);
                    Assert.Equal<int>(mallocSize, result);
                }
            }
            foreach (var option in options)
            {
                var mallocSize = heapSize / 2;
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, null, null, GetSandboxConfiguration()))
                {
                    var functions = new MallocTests();

                    testOutput.WriteLine($"Testing MallocAndFree with GuestHeapSize:{heapSize} MallocSize:{mallocSize} option: {option}");
                    sandbox.BindGuestFunction("MallocAndFree", functions);
                    var result = functions.MallocAndFree!(mallocSize);
                    Assert.Equal<int>(mallocSize, result);
                }
            }
        }

        public static string GetPathForGuest(string guestBinaryFileName)
        {
            // Assume we are in debug mode unless the assembly was built in release mode
            var configuration = Assembly.GetExecutingAssembly()
                                        .GetCustomAttributes(false)
                                        .OfType<AssemblyConfigurationAttribute>()
                                        .FirstOrDefault()?.Configuration;

            if (string.IsNullOrWhiteSpace(configuration) || configuration.Equals("Debug", StringComparison.OrdinalIgnoreCase))
            {
                configuration = "debug";
            }
            else
            {
                configuration = "release";
            }

            var guestType = "rust";
            if (Environment.GetEnvironmentVariable("guesttype") == "c")
            {
                guestType = "c";
            }

            // Construct the path based on the configuration
            var basePath = AppDomain.CurrentDomain.BaseDirectory;
            var relativePath = Path.Combine("guests", guestType);
            var fullPath = Path.GetFullPath(Path.Combine(basePath, relativePath));

            return Path.Combine(fullPath, guestBinaryFileName);
        }


        public static SandboxRunOptions[] GetSandboxRunOptions()
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                if (Sandbox.IsHypervisorPresent())
                {
                    return new SandboxRunOptions[] {
                        SandboxRunOptions.RunFromGuestBinary,
                        SandboxRunOptions.None,
                        SandboxRunOptions.RunInProcess,
                        SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun,
                        SandboxRunOptions.RecycleAfterRun,
                        SandboxRunOptions.None | SandboxRunOptions.RecycleAfterRun
                    };
                }
                else
                {
                    return new SandboxRunOptions[] {
                        SandboxRunOptions.RunFromGuestBinary,
                        SandboxRunOptions.RunInProcess,
                        SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun
                    };
                }
            }
            return GetSandboxRunInHyperVisorOptions();
        }

        private static SandboxRunOptions[] GetSandboxRunInHyperVisorOptions()
        {
            if (Sandbox.IsHypervisorPresent())
            {
                return new SandboxRunOptions[] { SandboxRunOptions.None, SandboxRunOptions.RecycleAfterRun, SandboxRunOptions.None | SandboxRunOptions.RecycleAfterRun };
            }
            return new SandboxRunOptions[] { };
        }

        [Fact]
        public void Test_Bind_And_Expose_Methods()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "simpleguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            Assert.True(File.Exists(guestBinaryPath), $"Cannot find file {guestBinaryPath} to load into hyperlight");

            List<(Type type, List<string> exposedMethods, List<string> boundDelegates, List<string> exposedStaticMethods)> testData = new()
            {
                (typeof(NoExposedMembers), new(), new(), new()),
                (typeof(ExposedMembers), new() { "GetOne", "MethodWithArgs", "HostMethod1" }, new() { "GuestMethod1", "PrintOutput" }, new() { "GetTwo", "StaticMethodWithArgs" }),
                (typeof(ExposeStaticMethodsUsingAttribute), new(), new(), new() { "StaticGetInt", "HostMethod1", "StaticGetNothing"}),
                (typeof(ExposeInstanceMethodsUsingAttribute), new() { "HostMethod", "HostMethod1" }, new() { "GuestMethod" }, new()),
                (typeof(DontExposeSomeMembersUsingAttribute), new() { "GetOne", "MethodWithArgs" }, new(), new() { "GetTwo", "StaticMethodWithArgs" }),
            };
            foreach (var option in options)
            {
                foreach (var (type, exposedMethods, boundDelegates, exposedStaticMethods) in testData)
                {
                    foreach (var target in new object[] { type, GetInstance(type) })
                    {
                        using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, null, null, GetSandboxConfiguration()))
                        {
                            if (target is Type)
                            {
                                sandbox.ExposeHostMethods((Type)target!);
                            }
                            else
                            {
                                sandbox.ExposeAndBindMembers(target);
                            }
                            Assert.NotNull(sandbox);
                            List<string> methodNames = target is Type ? exposedStaticMethods : exposedMethods.Concat(exposedStaticMethods).ToList();
                            CheckExposedMethods(sandbox, methodNames, target);
                            if (target is not Type)
                            {
                                CheckBoundDelegates(target!, boundDelegates);
                            }
                        }
                    }
                }

                var delegateNames = new List<string> { "GuestMethod", "GuestMethod1" };
                var hostMethods = new List<string> { "HostMethod", "HostMethod1" };
                using (var sandbox = new Sandbox(guestBinaryPath, option, null, null, null, null, GetSandboxConfiguration()))
                {
                    var instance = new CallbackTestMembers();
                    foreach (var delegateName in delegateNames)
                    {
                        sandbox.BindGuestFunction(delegateName, instance);
                    }
                    foreach (var hostMethod in hostMethods)
                    {
                        sandbox.ExposeHostMethod(hostMethod, instance);
                    }
                    CheckBoundDelegates(instance!, delegateNames);
                    CheckExposedMethods(sandbox, hostMethods, instance);
                }
            }
        }

        private void CheckBoundDelegates(object target, List<string> delegateNames)
        {
            foreach (var d in delegateNames!)
            {
                var fieldInfo = target.GetType().GetField(d, BindingFlags.Public | BindingFlags.Instance);
                Assert.NotNull(fieldInfo);
                Assert.True(typeof(Delegate).IsAssignableFrom(fieldInfo!.FieldType));
                Assert.NotNull(fieldInfo.GetValue(target));
            }
        }

        private void CheckExposedMethods(Sandbox sandbox, List<string> methodNames, object target)
        {
            // TODO: Need to find a new way of doing this now that the list of exposed methods is no longer available in the Sandbox
            // This may be easier to do once the invocation of host method is moved to flatbuffers and validation of Host method is done in the GuestLibrary
        }

        [Fact]
        private void Test_SandboxInit()
        {
            var options = GetSandboxRunOptions();
            var guestBinaryFileName = "callbackguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            Assert.True(File.Exists(guestBinaryPath), $"Cannot find file {guestBinaryPath} to load into hyperlight");

            List<(Type type, List<string> exposedMethods, List<string> boundDelegates, List<string> exposedStaticMethods, List<(string delegateName, int returnValue, string expectedOutput, object[]? args)> expectedResults)> testData = new()
            {
                (typeof(ExposedMembers), new() { "GetOne", "MethodWithArgs", "HostMethod1" }, new() { "GuestMethod1", "PrintOutput" }, new() { "GetTwo", "StaticMethodWithArgs" }, new()
                {
                    ("GuestMethod1", 77, "Host Method 1 Received: Hello from GuestFunction1, Hello from Init from Guest", new object[] { "Hello from Init" }),
                    ("PrintOutput", 15, "Hello from Init", new object[] { "Hello from Init" }),
                }),
                (typeof(ExposeStaticMethodsUsingAttribute), new(), new(), new() { "StaticGetInt", "HostMethod1" }, new()),
                (typeof(ExposeInstanceMethodsUsingAttribute), new() { "HostMethod", "HostMethod1" }, new() { "GuestMethod", "PrintOutput" }, new(), new()
                {
                    ("GuestMethod", 67, "Host Received: Hello from GuestFunction, Hello from Init from Guest", new object[] { "Hello from Init" }),
                    ("PrintOutput", 21, "Hello again from Init", new object[] { "Hello again from Init" }),
                }),
                (typeof(DontExposeSomeMembersUsingAttribute), new() { "GetOne", "MethodWithArgs" }, new(), new() { "GetTwo", "StaticMethodWithArgs" }, new()),
            };

            Action<ISandboxRegistration>? func;
            StringWriter output;
            Sandbox sandbox;

            foreach (var option in options)
            {
                foreach (var (type, exposedMethods, boundDelegates, exposedStaticMethods, expectedResults) in testData)
                {
                    output = new StringWriter();
                    var numberOfCalls = 0;
                    foreach (var target in new object[] { type, GetInstance(type) })
                    {
                        // Call init explicity creating delegates and binding methods
                        if (target is Type)
                        {
                            func = (s) =>
                            {
                                foreach (var method in exposedStaticMethods)
                                {
                                    s.ExposeHostMethod(method, (Type)target);
                                }
                            };
                        }
                        else
                        {
                            func = (s) =>
                            {
                                foreach (var method in exposedMethods)
                                {
                                    s.ExposeHostMethod(method, target);
                                }
                                foreach (var method in exposedStaticMethods)
                                {
                                    s.ExposeHostMethod(method, target.GetType());
                                }
                                foreach (var boundDelegate in boundDelegates)
                                {
                                    s.BindGuestFunction(boundDelegate, target);
                                }
                                foreach (var expectedResult in expectedResults)
                                {
                                    InvokeMethod(target, expectedResult.delegateName, expectedResult.returnValue, expectedResult.expectedOutput, expectedResult.args, output, ref numberOfCalls);
                                }
                            };
                        }

                        sandbox = new Sandbox(guestBinaryPath, option, func, output, null, null, GetSandboxConfiguration());
                        if (target is Type)
                        {
                            CheckExposedMethods(sandbox, exposedStaticMethods, (Type)target);
                        }
                        else
                        {
                            Assert.Equal<int>(expectedResults.Count, numberOfCalls);
                        }
                        sandbox.Dispose();
                    }

                    foreach (var target in new object[] { type, GetInstance(type) })
                    {

                        // Pass instance/type via constructor and call methods in init.

                        if (target is Type)
                        {
                            func = null;
                        }
                        else
                        {
                            func = (s) =>
                            {
                                s.ExposeAndBindMembers(target);
                                foreach (var expectedResult in expectedResults)
                                {
                                    InvokeMethod(target, expectedResult.delegateName, expectedResult.returnValue, expectedResult.expectedOutput, expectedResult.args, output, ref numberOfCalls);
                                }
                            };
                        }

                        numberOfCalls = 0;
                        output = new StringWriter();
                        sandbox = new Sandbox(guestBinaryPath, option, func, output, null, null, GetSandboxConfiguration());
                        if (target is Type)
                        {
                            sandbox.ExposeHostMethods((Type)target);
                            CheckExposedMethods(sandbox, exposedStaticMethods, (Type)target);
                        }
                        else
                        {
                            Assert.Equal<int>(expectedResults.Count, numberOfCalls);
                        }
                        sandbox.Dispose();

                    }
                }
            }
        }

        private void InvokeMethod(object target, string delegateName, int returnValue, string expectedOutput, object[]? args, StringWriter output, ref int numberOfCalls)
        {
            numberOfCalls++;
            var del = GetDelegate(target, delegateName);
            var result = del.DynamicInvoke(args);
            Assert.NotNull(result);
            Assert.Equal(returnValue, (int)result!);
            var builder = output.GetStringBuilder();
            Assert.Equal(expectedOutput, builder.ToString());
            builder.Remove(0, builder.Length);
        }

        private static Delegate GetDelegate(object target, string delegateName)
        {
            var fieldInfo = target.GetType().GetField(delegateName, BindingFlags.Public | BindingFlags.Instance);
            Assert.NotNull(fieldInfo);
            Assert.True(typeof(Delegate).IsAssignableFrom(fieldInfo!.FieldType));
            var fieldValue = fieldInfo.GetValue(target);
            Assert.NotNull(fieldValue);
            var del = (Delegate)fieldValue!;
            Assert.NotNull(del);
            return del;
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Loads_Windows_Exe(TestData testData)
        {
            SandboxRunOptions[] options = {
                SandboxRunOptions.RunFromGuestBinary,
                SandboxRunOptions.RunFromGuestBinary | SandboxRunOptions.RunInProcess };
            foreach (var option in options)
            {
                RunTests(testData, option, SimpleTest);
            }
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Loads_Windows_Exe_Concurrently(TestData testData)
        {
            SandboxRunOptions[] options = {
                SandboxRunOptions.RunFromGuestBinary,
                SandboxRunOptions.RunFromGuestBinary | SandboxRunOptions.RunInProcess
            };
            foreach (var option in options)
            {
                var correlationId = Guid.NewGuid().ToString("N");
                var output1 = new StringWriter();
                var sboxBuilder1 = new SandboxBuilder()
                    .WithConfig(GetSandboxConfiguration())
                    .WithGuestBinaryPath(testData.GuestBinaryPath)
                    .WithRunOptions(option)
                    .WithWriter(output1);
                var sandbox1 = sboxBuilder1.Build();
                var builder = output1.GetStringBuilder();
                SimpleTest(sandbox1, testData, output1, builder);
                var output2 = new StringWriter();
                Sandbox? sandbox2 = null;
                var sboxBuilder2 = new SandboxBuilder()
                    .WithConfig(GetSandboxConfiguration())
                    .WithGuestBinaryPath(testData.GuestBinaryPath)
                    .WithRunOptions(option)
                    .WithWriter(output2)
                    .WithCorrelationId(correlationId);
                var ex = Record.Exception(() => sandbox2 = sboxBuilder2.Build());
                Assert.NotNull(ex);
                Assert.IsType<HyperlightException>(ex);
                Assert.StartsWith("Only one instance of Sandbox is allowed when running from guest binary", ex.Message);
                Assert.Contains($"CorrelationId: {correlationId} Source: NativeHandleWrapperErrorExtensions", ex.Message);
                sandbox1.Dispose();
                sandbox2?.Dispose();
                output1.Dispose();
                output2.Dispose();
                using (var output = new StringWriter())
                {
                    var sboxBuilder = new SandboxBuilder()
                        .WithConfig(GetSandboxConfiguration())
                        .WithGuestBinaryPath(testData.GuestBinaryPath)
                        .WithRunOptions(option)
                        .WithWriter(output);
                    using (var sandbox = sboxBuilder.Build())
                    {
                        builder = output.GetStringBuilder();
                        SimpleTest(sandbox, testData, output, builder);
                    }
                }
            }
        }

        [FactSkipIfNotLinux]
        public void Test_Throws_InProcess_On_Linux()
        {
            SandboxRunOptions[] options = { SandboxRunOptions.RunInProcess, SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun };
            foreach (var option in options)
            {
                var guestBinaryFileName = "simpleguest.exe";
                var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
                var correlationId = Guid.NewGuid().ToString("N");
                var ex = Record.Exception(() =>
                {
                    var bld = new SandboxBuilder()
                    .WithGuestBinaryPath(guestBinaryPath)
                    .WithRunOptions(option)
                    .WithCorrelationId(correlationId)
                    .WithConfig(GetSandboxConfiguration());
                    using (var sandbox = bld.Build())
                    {
                        var guestMethods = new SimpleTestMembers();
                        sandbox.BindGuestFunction("PrintOutput", guestMethods);
                        var result = guestMethods.PrintOutput!("This will throw an exception");
                    }
                });
                Assert.NotNull(ex);
                Assert.IsType<HyperlightException>(ex);
                Assert.Equal($"In-process mode is currently only available on Windows CorrelationId: {correlationId} Source: NativeHandleWrapperErrorExtensions", ex.Message);
            }
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Runs_InProcess(TestData testData)
        {
            SandboxRunOptions[] options = {
                SandboxRunOptions.RunInProcess,
                SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun
            };
            foreach (var option in options)
            {
                RunTests(testData, option, SimpleTest);
            }
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Runs_InProcess_Concurrently(TestData testData)
        {
            Parallel.For(0, testData.NumberOfParallelTests, (t) =>
            {
                SandboxRunOptions[] options = {
                    SandboxRunOptions.RunInProcess,
                    SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun
                };
                foreach (var option in options)
                {
                    RunTests(testData, option, SimpleTest);
                }
            });
        }

        [TheorySkipIfHyperVisorNotPresent]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Runs_InHyperVisor(TestData testData)
        {
            SandboxRunOptions[] options = {
                SandboxRunOptions.None,
                SandboxRunOptions.None | SandboxRunOptions.RecycleAfterRun
            };
            foreach (var option in options)
            {
                RunTests(testData, option, SimpleTest);
            }

            // Run tests using constructors without options
            foreach (var instanceOrType in testData.TestInstanceOrTypes())
            {
                RunTest(testData, instanceOrType, SimpleTest);
            }
        }

        [TheorySkipIfHyperVisorNotPresent]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Runs_InHyperVisor_Concurrently(TestData testData)
        {
            Parallel.For(0, testData.NumberOfParallelTests, (t) =>
            {
                SandboxRunOptions[] options = { SandboxRunOptions.None, SandboxRunOptions.None | SandboxRunOptions.RecycleAfterRun };
                foreach (var option in options)
                {
                    RunTests(testData, option, SimpleTest);
                }

                // Run tests using constructors without options
                foreach (var instanceOrType in testData.TestInstanceOrTypes())
                {
                    RunTest(testData, instanceOrType, SimpleTest);
                }
            });

        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetCallbackTestData))]
        public void Test_Loads_Windows_Exe_With_Callback(TestData testData)
        {
            SandboxRunOptions[] options = {
                SandboxRunOptions.RunFromGuestBinary,
                SandboxRunOptions.RunFromGuestBinary | SandboxRunOptions.RunInProcess
            };
            foreach (var option in options)
            {
                RunTests(testData, option, CallbackTest);
            }
        }

        [Fact]
        public void Test_RecycleAfterRun()
        {

            var options = Array.Empty<SandboxRunOptions>();
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                if (Sandbox.IsHypervisorPresent())
                {
                    options = new SandboxRunOptions[] { SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun, SandboxRunOptions.None | SandboxRunOptions.RecycleAfterRun };
                }
                else
                {
                    options = new SandboxRunOptions[] { SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun };
                }
            }
            else
            {
                if (Sandbox.IsHypervisorPresent())
                {
                    options = new SandboxRunOptions[] { SandboxRunOptions.None | SandboxRunOptions.RecycleAfterRun };
                }
            }

            var guestBinaryFileName = "callbackguest.exe";
            var guestBinaryPath = GetPathForGuest(guestBinaryFileName);
            foreach (var option in options)
            {
                var stopWatch = Stopwatch.StartNew();

                // See https://github.com/deislabs/hyperlight/issues/947 for why we are using 100 iterations on KVM.

                var numberOfIterations = 25000;
                if (IsRunningOnKVM)
                {
                    numberOfIterations = 100;
                }
                var instance = new ExposeInstanceMethodsUsingAttribute();
                using (var output = new StringWriter())
                {
                    var correlationId = Guid.NewGuid().ToString("N");
                    using (var sandbox = new Sandbox(guestBinaryPath, option, null, output, correlationId, null, GetSandboxConfiguration()))
                    {
                        sandbox.ExposeAndBindMembers(instance);
                        var builder = output.GetStringBuilder();
                        for (var i = 0; i < numberOfIterations; i++)
                        {
                            builder.Remove(0, builder.Length);
                            var message = $"Hello from RecycleAfterRun Instance {numberOfIterations}";
                            var expectedMessage = $"Host Received: Hello from GuestFunction, {message} from Guest";
                            var result = sandbox.CallGuest<int>(() =>
                            {
                                return instance.GuestMethod!(message);
                            });
                            Assert.Equal<int>(expectedMessage.Length, (int)result!);
                            Assert.Equal(expectedMessage, builder.ToString());
                        }
                    }
                }
                stopWatch.Stop();
                testOutput.WriteLine($"RecycleAfterRun Test {numberOfIterations} iterations with options {option} in {stopWatch.Elapsed.TotalMilliseconds} ms");
            }
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetCallbackTestData))]
        public void Test_Loads_Windows_Exe_With_Callback_Concurrently(TestData testData)
        {
            SandboxRunOptions[] options = {
                SandboxRunOptions.RunFromGuestBinary,
                SandboxRunOptions.RunFromGuestBinary | SandboxRunOptions.RunInProcess
            };
            foreach (var option in options)
            {
                foreach (var instanceOrType in testData.TestInstanceOrTypes())
                {
                    var output1 = new StringWriter();
                    var builder = output1.GetStringBuilder();
                    var correlationId1 = Guid.NewGuid().ToString("N");
                    using var sandbox1 = new Sandbox(testData.GuestBinaryPath, option, null, output1, correlationId1, null, GetSandboxConfiguration());
                    if (instanceOrType is not null)
                    {
                        if (instanceOrType is Type)
                        {
                            sandbox1.ExposeHostMethods((Type)instanceOrType);
                        }
                        else
                        {
                            sandbox1.ExposeAndBindMembers(instanceOrType);
                        }
                    }
                    CallbackTest(sandbox1, testData, output1, builder, instanceOrType, correlationId1);
                    var output2 = new StringWriter();
                    Sandbox? sandbox2 = null;
                    object? instanceOrType1;
                    if (instanceOrType is not null && instanceOrType is not Type)
                    {
                        instanceOrType1 = GetInstance(instanceOrType.GetType());
                    }
                    else
                    {
                        instanceOrType1 = instanceOrType;
                    }
                    var correlationId2 = Guid.NewGuid().ToString("N");
                    var ex = Record.Exception(() => sandbox2 = new Sandbox(testData.GuestBinaryPath, option, null, output2, correlationId2, null, GetSandboxConfiguration()));
                    Assert.NotNull(ex);
                    Assert.IsType<HyperlightException>(ex);
                    Assert.StartsWith("Only one instance of Sandbox is allowed when running from guest binary", ex.Message);
                    Assert.Contains($"CorrelationId: {correlationId2} Source: NativeHandleWrapperErrorExtensions", ex.Message);
                    sandbox1.Dispose();
                    sandbox2?.Dispose();
                    output1.Dispose();
                    output2.Dispose();
                    using (var output = new StringWriter())
                    {
                        var correlationId = Guid.NewGuid().ToString("N");
                        builder = output.GetStringBuilder();
                        using (var sandbox = new Sandbox(testData.GuestBinaryPath, option, null, output, correlationId, null, GetSandboxConfiguration()))
                        {
                            if (instanceOrType1 is not null)
                            {
                                if (instanceOrType1 is Type)
                                {
                                    sandbox.ExposeHostMethods((Type)instanceOrType1);
                                }
                                else
                                {
                                    sandbox.ExposeAndBindMembers(instanceOrType1);
                                }
                            }
                            CallbackTest(sandbox, testData, output, builder, instanceOrType1, correlationId);
                        }
                    }
                }
            }
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetCallbackTestData))]
        public void Test_Runs_InProcess_With_Callback(TestData testData)
        {
            SandboxRunOptions[] options = {
                SandboxRunOptions.RunInProcess,
                SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun
            };
            foreach (var option in options)
            {
                RunTests(testData, option, CallbackTest);
            }
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetCallbackTestData))]
        public void Test_Runs_InProcess_With_Callback_Concurrently(TestData testData)
        {
            Parallel.For(0, testData.NumberOfParallelTests, (t) =>
            {
                SandboxRunOptions[] options = { SandboxRunOptions.RunInProcess };
                foreach (var option in options)
                {
                    RunTests(testData, option, CallbackTest);
                }
            });
        }

        [TheorySkipIfHyperVisorNotPresent]
        [MemberData(nameof(GetCallbackTestData))]
        public void Test_Runs_InHyperVisor_With_Callback(TestData testData)
        {
            SandboxRunOptions[] options = { SandboxRunOptions.None, SandboxRunOptions.None | SandboxRunOptions.RecycleAfterRun };
            foreach (var option in options)
            {
                RunTests(testData, option, CallbackTest);
            }

            // Run tests using constructors without options
            foreach (var instanceOrType in testData.TestInstanceOrTypes())
            {
                RunTest(testData, instanceOrType, CallbackTest);
            }
        }


        [TheorySkipIfHyperVisorNotPresent]
        [MemberData(nameof(GetCallbackTestData))]
        public void Test_Runs_InHyperVisor_With_Callback_Concurrently(TestData testData)
        {
            Parallel.For(0, testData.NumberOfParallelTests, (t) =>
            {
                SandboxRunOptions[] options = { SandboxRunOptions.None, SandboxRunOptions.None | SandboxRunOptions.RecycleAfterRun };
                foreach (var option in options)
                {
                    RunTests(testData, option, CallbackTest);
                }

                // Run tests using constructors without options
                foreach (var instanceOrType in testData.TestInstanceOrTypes())
                {
                    RunTest(testData, instanceOrType, CallbackTest);
                }
            });
        }

        private void RunTests(TestData testData, SandboxRunOptions options, Action<Sandbox, TestData, StringWriter, StringBuilder, object?, string, bool> test)
        {
            if (testData.TestInstanceOrTypes().Length > 0)
            {
                foreach (var instanceOrType in testData.TestInstanceOrTypes())
                {
                    var correlationId = Guid.NewGuid().ToString("N");
                    RunTest(testData, options, instanceOrType, correlationId, test);
                }
            }
            else
            {
                RunTest(testData, options, test);
            }
        }

        private void RunTest(TestData testData, SandboxRunOptions sandboxRunOptions, object? instanceOrType, string correlationId, Action<Sandbox, TestData, StringWriter, StringBuilder, object?, string, bool> test)
        {
            using (var output = new StringWriter())
            {
                using (var sandbox = new Sandbox(
                    testData.GuestBinaryPath,
                    sandboxRunOptions,
                    null,
                    output,
                    correlationId,
                    null,
                    GetSandboxConfiguration()
                ))
                {
                    if (instanceOrType is not null)
                    {
                        if (instanceOrType is Type)
                        {
                            sandbox.ExposeHostMethods((Type)instanceOrType);
                        }
                        else
                        {
                            sandbox.ExposeAndBindMembers(instanceOrType);
                        }
                    }
                    var numberOfIterations = sandboxRunOptions.HasFlag(SandboxRunOptions.RecycleAfterRun) ? testData.NumberOfIterations : 1;
                    var builder = output.GetStringBuilder();
                    var explicitlyReset = false;
                    for (var i = 0; i < numberOfIterations; i++)
                    {
                        if (numberOfIterations > 0 && i > 0)
                        {
                            explicitlyReset = ShouldReset();
                        }
                        builder.Remove(0, builder.Length);
                        test(sandbox, testData, output, builder, instanceOrType, correlationId, explicitlyReset);
                    }
                }
            }
        }

        private bool ShouldReset()
        {
            var min = 0;
            var max = 1000;
            var random = new Random();
            var result = random.NextInt64(min, max + 1) % 2 == 0;
            testOutput.WriteLine($"Explicit Reset: {result}");
            return result;
        }

        private void RunTest(TestData testData, SandboxRunOptions sandboxRunOptions, Action<Sandbox, TestData, StringWriter, StringBuilder, object?, string, bool> test)
        {
            using (var output = new StringWriter())
            {
                var correlationId = Guid.NewGuid().ToString("N");

                var sboxBuilder = new SandboxBuilder()
                    .WithConfig(GetSandboxConfiguration())
                    .WithGuestBinaryPath(testData.GuestBinaryPath)
                    .WithRunOptions(sandboxRunOptions)
                    .WithWriter(output)
                    .WithCorrelationId(correlationId);
                using (var sandbox = sboxBuilder.Build())
                {
                    var numberOfIterations = sandboxRunOptions.HasFlag(SandboxRunOptions.RecycleAfterRun) ? testData.NumberOfIterations : 1;
                    var builder = output.GetStringBuilder();
                    var explicitlyReset = false;
                    for (var i = 0; i < numberOfIterations; i++)
                    {
                        if (numberOfIterations > 0 && i > 0)
                        {
                            explicitlyReset = ShouldReset();
                        }
                        builder.Remove(0, builder.Length);
                        test(sandbox, testData, output, builder, null, correlationId, explicitlyReset);
                    }
                }
            }
        }

        private void RunTest(TestData testData, object? instanceOrType, Action<Sandbox, TestData, StringWriter, StringBuilder, object?, string, bool> test)
        {
            using (var output = new StringWriter())
            {
                var correlationId = Guid.NewGuid().ToString("N");
                var sboxBuilder = new SandboxBuilder()
                    .WithConfig(GetSandboxConfiguration())
                    .WithGuestBinaryPath(testData.GuestBinaryPath)
                    .WithWriter(output)
                    .WithCorrelationId(correlationId);
                using (var sandbox = sboxBuilder.Build())
                {
                    if (instanceOrType is not null)
                    {
                        if (instanceOrType is Type)
                        {
                            sandbox.ExposeHostMethods((Type)instanceOrType);
                        }
                        else
                        {
                            sandbox.ExposeAndBindMembers(instanceOrType);
                        }
                    }
                    var builder = output.GetStringBuilder();
                    test(sandbox, testData, output, builder, instanceOrType, correlationId, false);
                }
            }
        }

        private void SimpleTest(Sandbox sandbox, TestData testData, StringWriter output, StringBuilder builder, object? _ = null, string correlationId = "", bool explicitlyReset = false)
        {
            var guestMethods = new SimpleTestMembers();
            sandbox.BindGuestFunction("PrintOutput", guestMethods);
            sandbox.BindGuestFunction("Echo", guestMethods);
            sandbox.BindGuestFunction("GetSizePrefixedBuffer", guestMethods);
            if (explicitlyReset)
            {
                sandbox.RestoreState();
            }
            var result1 = sandbox.CallGuest<string>(() =>
            {
                var result = guestMethods.PrintOutput!(testData.ExpectedOutput);
                Assert.Equal<int>(testData.ExpectedReturnValue, result);
                Assert.Equal(testData.ExpectedOutput, builder.ToString());
                var array = new byte[] { 1, 2, 3, 4, 5, 6 };
                var result2 = guestMethods.GetSizePrefixedBuffer!(array, array.Length);
                Assert.Equal(result2, array);
                return guestMethods.Echo!(testData.ExpectedOutput);
            });
            Assert.Equal(testData.ExpectedOutput, result1);
        }

        private void CallbackTest(Sandbox sandbox, TestData testData, StringWriter output, StringBuilder builder, object? typeorinstance, string correlationId, bool explicitlyReset = false)
        {

            if (typeorinstance == null)
            {
                // TODO: Enables this by handling errors correctly in Sandbox when crossing managed native bounary.
                var guestMethods = new CallbackTestMembers();
                sandbox.BindGuestFunction("GuestMethod", guestMethods);
                var ex = Record.Exception(() =>
                {
                    var result = guestMethods.GuestMethod!(testData.ExpectedOutput);
                });
                Assert.NotNull(ex);
                Assert.IsType<HyperlightException>(ex);
                Assert.Equal($"GuestError:Host Function Not Found: HostMethod CorrelationId: {correlationId} Source: Sandbox", ex.Message);
            }
            else
            {
                if (typeorinstance is Type)
                {
                    CallGuestMethod1(sandbox, testData, builder, explicitlyReset);
                }
                else
                {
                    var message = "Hello from CallbackTest";
                    var expectedMessage = string.Format(testData.ExpectedOutput, message);
                    var fieldInfo = GetDelegateFieldInfo(typeorinstance, "GuestMethod1");
                    if (fieldInfo != null)
                    {
                        if (explicitlyReset)
                        {
                            sandbox.RestoreState();
                        }
                        var fieldValue = fieldInfo.GetValue(typeorinstance);
                        Assert.NotNull(fieldValue);
                        var del = (Delegate)fieldValue!;
                        Assert.NotNull(del);
                        var args = new object[] { message };
                        var result = sandbox.CallGuest<int>(() =>
                        {
                            var result = del!.DynamicInvoke(args);
                            Assert.NotNull(result);
                            return (int)result!;
                        });
                        Assert.Equal<int>(expectedMessage.Length, (int)result!);
                        Assert.Equal(expectedMessage, builder.ToString());
                    }
                    else
                    {
                        // This should be updated to call both tests for now it chooses one as RecycleAfterRun may not be set.
                        if (ShouldCallMethod1())
                        {
                            testOutput.WriteLine("Calling GuestMethod1");
                            CallGuestMethod1(sandbox, testData, builder, explicitlyReset);
                        }
                        else
                        {
                            testOutput.WriteLine("Calling GuestMethod4");
                            CallGuestMethod4(sandbox, testData, builder, output, explicitlyReset);
                        }
                    }
                }
            }

            static void CallGuestMethod1(Sandbox sandbox, TestData testData, StringBuilder builder, bool explicitlyReset)
            {
                var guestMethods = new CallbackTestMembers();
                sandbox.BindGuestFunction("GuestMethod1", guestMethods);
                sandbox.BindGuestFunction("PrintOutput", guestMethods);
                sandbox.ExposeHostMethod("HostMethod1", guestMethods);
                var message = "Hello from CallbackTest";
                var expectedMessage = string.Format(testData.ExpectedOutput, message);
                if (explicitlyReset)
                {
                    sandbox.RestoreState();
                }
                var result = sandbox.CallGuest<int>(() => { return guestMethods.GuestMethod1!(message); });
                Assert.Equal<int>(testData.ExpectedReturnValue, result);
                Assert.Equal(expectedMessage, builder.ToString());

            }

            static void CallGuestMethod4(Sandbox sandbox, TestData testData, StringBuilder builder, StringWriter output, bool explicitlyReset)
            {
                var guestMethods = new CallbackTestMembers(output);
                sandbox.BindGuestFunction("GuestMethod4", guestMethods);
                sandbox.ExposeHostMethod("HostMethod4", guestMethods);
                var expectedMessage = "Hello from GuestFunction4";
                if (explicitlyReset)
                {
                    sandbox.RestoreState();
                }
                sandbox.CallGuest(() => guestMethods.GuestMethod4!());
                Assert.Equal(expectedMessage, builder.ToString());
            }
            bool ShouldCallMethod1()
            {
                var min = 0;
                var max = 2;
                var random = new Random();
                return random.Next(min, max) == 0;
            }
        }

        private FieldInfo? GetDelegateFieldInfo(object target, string MethodName)
        {
            var fieldInfo = target.GetType().GetField(MethodName, BindingFlags.Public | BindingFlags.Instance);
            if (fieldInfo is not null && typeof(Delegate).IsAssignableFrom(fieldInfo.FieldType))
            {
                return fieldInfo;
            }
            return null;
        }

        public static IEnumerable<object[]> GetSimpleTestData()
        {
            return new List<object[]>
            {
                new object[] { new TestData(typeof(NoExposedMembers), "simpleguest.exe") },
                new object[] { new TestData(typeof(ExposedMembers), "simpleguest.exe") },
                new object[] { new TestData(typeof(ExposeStaticMethodsUsingAttribute), "simpleguest.exe") },
                new object[] { new TestData(typeof(ExposeInstanceMethodsUsingAttribute), "simpleguest.exe") },
                new object[] { new TestData(typeof(DontExposeSomeMembersUsingAttribute), "simpleguest.exe") },
                new object[] { new TestData("simpleguest.exe") } ,
            };
        }

        public static IEnumerable<object[]> GetCallbackTestData()
        {
            return new List<object[]>
            {
                new object[] { new TestData(typeof(ExposedMembers), "callbackguest.exe", "Host Method 1 Received: Hello from GuestFunction1, {0} from Guest", 85,0,0,TestData.ExposeMembersToGuest.All) },
                new object[] { new TestData("callbackguest.exe", "Host Method 1 Received: Hello from GuestFunction1, {0} from Guest", 85, 0,0,TestData.ExposeMembersToGuest.Null) },
                new object[] { new TestData(typeof(ExposeStaticMethodsUsingAttribute), "callbackguest.exe", "Host Method 1 Received: Hello from GuestFunction1, {0} from Guest", 85,0,0, TestData.ExposeMembersToGuest.TypeAndNull) },
                new object[] { new TestData(typeof(ExposeInstanceMethodsUsingAttribute), "callbackguest.exe", "Host Method 1 Received: Hello from GuestFunction1, Hello from CallbackTest from Guest", 85, 0,0,TestData.ExposeMembersToGuest.InstanceAndNull) },
            };
        }

        private static object GetInstance(Type type)
        {

            var result = Activator.CreateInstance(type);
            Assert.NotNull(result);
            return result!;
        }

        private static T GetInstance<T>() => Activator.CreateInstance<T>();
        static bool RunningOnWindows => RuntimeInformation.IsOSPlatform(OSPlatform.Windows);

        private SandboxConfiguration GetSandboxConfiguration()
        {
            var errorMessageSize = GetErrorMessageSize();
            var functionDefinitionSize = GetFunctionDefinitionSize();
            var hostExceptionSize = GetHostExceptionSize();
            var inputDataSize = GetInputDataSize();
            var outputDataSize = GetOutputDataSize();
            var maxExecutionTime = GetMaxExecutionTime();
            var maxWaitForCancellation = GetMaxWaitForCancellation();

            // This is just so sometimes the defaults are used.

            if (errorMessageSize % 8 == 0 || functionDefinitionSize % 8 == 0 || hostExceptionSize % 8 == 0)
            {
                testOutput.WriteLine("Using Default Configuration");
                return new SandboxConfiguration();
            }
            testOutput.WriteLine($"Using Configuration: guestErrorMessageSize: {errorMessageSize} functionDefinitionSize: {functionDefinitionSize} hostExceptionSize: {hostExceptionSize} inputDataSize: {inputDataSize} outputDataSize: {outputDataSize}");
            return new SandboxConfiguration(
                guestErrorMessageSize: errorMessageSize,
                hostFunctionDefinitionSize: functionDefinitionSize,
                hostExceptionSize: hostExceptionSize,
                inputDataSize: inputDataSize,
                outputDataSize: outputDataSize,
                maxExecutionTime: maxExecutionTime,
                maxWaitForCancellation: maxWaitForCancellation
            );
        }

        private ulong GetErrorMessageSize()
        {
            var min = 256;
            var max = 1024;
            var random = new Random();
            return (ulong)random.NextInt64(min, max + 1);
        }

        private ulong GetFunctionDefinitionSize()
        {
            var min = 4096;
            var max = 8192;
            var random = new Random();
            return (ulong)random.NextInt64(min, max + 1);
        }

        private ulong GetHostExceptionSize()
        {
            var min = 1024;
            var max = 8192;
            var random = new Random();
            return (ulong)random.NextInt64(min, max + 1);
        }

        private ulong GetInputDataSize()
        {
            var min = 8182;
            var max = 65536;
            var random = new Random();
            return (ulong)random.NextInt64(min, max + 1);
        }

        private ulong GetOutputDataSize()
        {
            var min = 8182;
            var max = 65536;
            var random = new Random();
            return (ulong)random.NextInt64(min, max + 1);
        }

        private ushort GetMaxExecutionTime()
        {
            var min = 1200;
            var max = 2500;
            var random = new Random();
            return (ushort)random.Next(min, max + 1);
        }

        private byte GetMaxWaitForCancellation()
        {
            var min = 11;
            var max = 200;
            var random = new Random();
            return (byte)random.Next(min, max + 1);
        }
    }
}
