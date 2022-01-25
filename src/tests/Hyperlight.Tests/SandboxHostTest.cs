using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using Xunit;
using Xunit.Sdk;

namespace Hyperlight.Tests
{
    public class SandboxHostTest
    {
        public SandboxHostTest()
        {
            //TODO: implment skip for this
            Assert.True(Sandbox.IsSupportedPlatform, "Hyperlight Sandbox is not supported on this platform.");
        }

        public class TestData
        {
            public enum ExposeMembersToGuest
            {
                Instance,
                Type,
                All
            }
            public ulong Size => 1024 * 1024;
            public uint ExpectedReturnValue;
            public string ExpectedOutput;
            public string GuestBinaryPath { get; }
            public byte[] Workload;
            public int[] Args;
            public object? instance { get; }
            public object? CurrentInstance { get; set; } = null;
            public ExposeMembersToGuest ExposeMembers = ExposeMembersToGuest.All;

            public TestData(string guestBinaryFileName, byte[]? workload, int[] args, string expectedOutput = "Hello, World!!\n", uint? expectedReturnValue = null)
            {
                var path = AppDomain.CurrentDomain.BaseDirectory;
                var guestBinaryPath = Path.Combine(path, guestBinaryFileName);
                Assert.True(File.Exists(guestBinaryPath), $"Cannot find file {guestBinaryPath} to load into hyperlight");
                this.GuestBinaryPath = guestBinaryPath;
                this.Workload = workload ?? Array.Empty<byte>();
                this.Args = args;
                this.ExpectedOutput = expectedOutput;
                this.ExpectedReturnValue = expectedReturnValue ?? (uint)expectedOutput.Length;
            }

            public TestData(object instance, string guestBinaryFileName, byte[]? workload, int[] args, string expectedOutput = "Hello, World!!\n", uint? expectedReturnValue = null, ExposeMembersToGuest exposeMembers = ExposeMembersToGuest.All) : this(guestBinaryFileName, workload, args, expectedOutput, expectedReturnValue)
            {
                this.instance = instance;
                this.ExposeMembers = exposeMembers;
            }



            public object?[] TestInstanceOrTypes()
            {

                if (instance != null)
                {
                    return ExposeMembers switch
                    {
                        ExposeMembersToGuest.Instance => new object?[] { instance },
                        ExposeMembersToGuest.Type => new object?[] { instance.GetType() },
                        ExposeMembersToGuest.All => new object?[] { instance.GetType(), instance, null },
                        _ => Array.Empty<object?>(),
                    };
                }

                return Array.Empty<object?>();
            }

        }

        public class NoExposedMembers { }
        public class ExposedMembers
        {
            public int GetOne() => 1;
            public static int GetTwo() => 2;
            public int MethodWithArgs(string arg1, int arg2) { return arg1.Length + arg2; }
            public static int StaticMethodWithArgs(string arg1, int arg2) { return arg1.Length + arg2; }
            public static Func<string, int>? GuestMethod = null;
            public static int HostMethod(string msg)
            {
                return GuestMethod!($"Host Received: {msg} from Guest");
            }
        }

        [ExposeToGuest(false)]
        public class ExposeStaticMethodsUsingAttribute
        {
            public void GetNothing() { }
            public static void StaticGetNothing() { }
            public void GetNothingWithArgs(string arg1, int arg2) { }
            public static void StaticGetNothingWithArgs(string arg1, int arg2) { }
            [ExposeToGuest(true)]
            public static Func<string, int>? GuestMethod = null;
            [ExposeToGuest(true)]
            public static int HostMethod(string msg)
            {
                return GuestMethod!($"Host Received: {msg} from Guest");
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
                return GuestMethod!($"Host Received: {msg} from Guest");
            }
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

        [Theory]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Loads_Windows_Exe(TestData testData)
        {
            // TODO:Handle not running on Windows

            SandboxRunOptions[] options = { SandboxRunOptions.RunFromGuestBinary, SandboxRunOptions.RunFromGuestBinary | SandboxRunOptions.RunInProcess };
            foreach (var option in options)
            {
                RunTests(testData, option, SimpleTest);
            }

        }

        [Theory]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Runs_InProcess(TestData testData)
        {
            SandboxRunOptions[] options = { SandboxRunOptions.RunInProcess, SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun };
            foreach (var option in options)
            {
                RunTests(testData, option, SimpleTest);
            }
        }

        [Theory]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Runs_InHyperVisor(TestData testData)
        {
            //TODO: handle Hypervisor not present
            Assert.True(Sandbox.IsHypervisorPresent(), "Hypervisor is not present on this platform.");
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
        }

        [Theory]
        [MemberData(nameof(GetCallbackTestData))]
        public void Test_Loads_Windows_Exe_With_Callback(TestData testData)
        {
            // TODO:Handle not running on Windows

            SandboxRunOptions[] options = { SandboxRunOptions.RunFromGuestBinary, SandboxRunOptions.RunFromGuestBinary | SandboxRunOptions.RunInProcess };
            foreach (var option in options)
            {
                RunTests(testData, option, CallbackTest);
            }

        }

        // 
        // TODO: The test below does not pass null instanceOrType to the Sandbox constructor as if this is done then when the outb method throws
        // a "Could not find host function name {functionName}. GuestInterfaceGlue is null" exception the test process crashes.
        // need to investigate this.
        //

        [Theory]
        [MemberData(nameof(GetCallbackTestDataNoNullInstanceOrType))]
        public void Test_Runs_InProcess_With_Callback(TestData testData)
        {
            SandboxRunOptions[] options = { SandboxRunOptions.RunInProcess, SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun };
            foreach (var option in options)
            {
                RunTests(testData, option, CallbackTest);
            }
        }

        [Theory]
        [MemberData(nameof(GetCallbackTestData))]
        public void Test_Runs_InHyperVisor_With_Callback(TestData testData)
        {
            //TODO: handle Hypervisor not present
            Assert.True(Sandbox.IsHypervisorPresent(), "Hypervisor is not present on this platform.");
            SandboxRunOptions[] options = { SandboxRunOptions.None, SandboxRunOptions.None | SandboxRunOptions.RecycleAfterRun };
            foreach (var option in options)
            {
                RunTests(testData, option, CallbackTest);
            }

            // Run tests using constructors without options
            foreach (var instanceOrType in testData.TestInstanceOrTypes())
            {
                testData.CurrentInstance = instanceOrType;
                RunTest(testData, instanceOrType, CallbackTest);
            }
        }

        private void RunTests(TestData testData, SandboxRunOptions options, Action<Sandbox, TestData> test)
        {
            if (testData.TestInstanceOrTypes().Length > 0)
            {
                foreach (var instanceOrType in testData.TestInstanceOrTypes())
                {
                    testData.CurrentInstance = instanceOrType;
                    RunTest(testData, options, instanceOrType, test);
                }
            }
            else
            {
                testData.CurrentInstance = null;
                RunTest(testData, options, test);
            }
        }

        private void RunTest(TestData testData, SandboxRunOptions sandboxRunOptions, object? instanceOrType, Action<Sandbox, TestData> test)
        {
            using var sandbox = new Sandbox(testData.Size, testData.GuestBinaryPath, sandboxRunOptions, instanceOrType);
            test(sandbox, testData);
        }

        private void RunTest(TestData testData, SandboxRunOptions sandboxRunOptions, Action<Sandbox, TestData> test)
        {
            using var sandbox = new Sandbox(testData.Size, testData.GuestBinaryPath, sandboxRunOptions);
            test(sandbox, testData);
        }

        private void RunTest(TestData testData, object? instanceOrType, Action<Sandbox, TestData> test)
        {
            using var sandbox = instanceOrType == null ? new Sandbox(testData.Size, testData.GuestBinaryPath) : new Sandbox(testData.Size, testData.GuestBinaryPath, instanceOrType);
            test(sandbox, testData);
        }

        private void RunTest(TestData testData, Action<Sandbox, TestData> test)
        {
            using var sandbox = new Sandbox(testData.Size, testData.GuestBinaryPath);
            test(sandbox, testData);
        }

        private void SimpleTest(Sandbox sandbox, TestData testData)
        {
            using var output = new StringWriter();
            Console.SetOut(output);
            (var result, _, _) = sandbox.Run(testData.Workload, testData.Args[0], testData.Args[1], testData.Args[2]);
            Assert.Equal<uint>(testData.ExpectedReturnValue, result);
            Assert.Equal(testData.ExpectedOutput, output.ToString());
        }

        private void CallbackTest(Sandbox sandbox, TestData testData)
        {
            using var output = new StringWriter();
            Console.SetOut(output);

            if (testData.CurrentInstance == null)
            {

                var ex = Record.Exception(() => sandbox.Run(testData.Workload, testData.Args[0], testData.Args[1], testData.Args[2]));
                Assert.NotNull(ex);
                Assert.IsType<Exception>(ex);
                Assert.Equal("Could not find host function name HostMethod. GuestInterfaceGlue is null", ex.Message);
            }
            else
            {
                (var result, _, _) = sandbox.Run(testData.Workload, testData.Args[0], testData.Args[1], testData.Args[2]);
                Assert.Equal<uint>(testData.ExpectedReturnValue, result);
                Assert.Equal(testData.ExpectedOutput, output.ToString());
            }
        }

        public static IEnumerable<object[]> GetSimpleTestData()
        {
            return new List<object[]>
            {
                new object[] { new TestData(new NoExposedMembers(), "simpleguest.exe", null, new int[3] { 0, 0, 0 }) },
                new object[] { new TestData(new ExposedMembers(), "simpleguest.exe", null, new int[3] { 0, 0, 0 }) },
                new object[] { new TestData(new ExposeStaticMethodsUsingAttribute(), "simpleguest.exe", null, new int[3] { 0, 0, 0 }) },
                new object[] { new TestData(new ExposeInstanceMethodsUsingAttribute(), "simpleguest.exe", null, new int[3] { 0, 0, 0 }) },
                new object[] { new TestData(new DontExposeSomeMembersUsingAttribute(), "simpleguest.exe", null, new int[3] { 0, 0, 0 }) },
                new object[] { new TestData("simpleguest.exe", null, new int[3] { 0, 0, 0 }) } ,
            };
        }

        public static IEnumerable<object[]> GetCallbackTestData()
        {
            return new List<object[]>
            {
                new object[] { new TestData(new ExposedMembers(), "callbackguest.exe", null, new int[3] { 0, 0, 0 }, "Hello from GuestFunction, Host Received: Hello, World!! from Guest!!.\n") },
                new object[] { new TestData(new ExposeStaticMethodsUsingAttribute(), "callbackguest.exe", null, new int[3] { 0, 0, 0 }, "Hello from GuestFunction, Host Received: Hello, World!! from Guest!!.\n", 70, TestData.ExposeMembersToGuest.Type) },
                new object[] { new TestData(new ExposeInstanceMethodsUsingAttribute(), "callbackguest.exe", null, new int[3] { 0, 0, 0 }, "Hello from GuestFunction, Host Received: Hello, World!! from Guest!!.\n", 70, TestData.ExposeMembersToGuest.Instance) },
            };
        }

        public static IEnumerable<object[]> GetCallbackTestDataNoNullInstanceOrType()
        {
            return new List<object[]>
            {
                new object[] { new TestData(new ExposeStaticMethodsUsingAttribute(), "callbackguest.exe", null, new int[3] { 0, 0, 0 }, "Hello from GuestFunction, Host Received: Hello, World!! from Guest!!.\n", 70, TestData.ExposeMembersToGuest.Type) },
                new object[] { new TestData(new ExposeInstanceMethodsUsingAttribute(), "callbackguest.exe", null, new int[3] { 0, 0, 0 }, "Hello from GuestFunction, Host Received: Hello, World!! from Guest!!.\n", 70, TestData.ExposeMembersToGuest.Instance) },
            };
        }

        static bool RunningOnWindows => RuntimeInformation.IsOSPlatform(OSPlatform.Windows);
    }
}
