using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using Xunit;

namespace Hyperlight.Tests
{
    public class SandboxHostTest
    {
        readonly int numberOfParallelTests = 100;
        public SandboxHostTest()
        {
            //TODO: implment skip for this
            Assert.True(Sandbox.IsSupportedPlatform, "Hyperlight Sandbox is not supported on this platform.");

            //TODO: Need to test and fix multi instance on Linux
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                numberOfParallelTests = 1;
            }
        }
        public class TestData
        {
            public enum ExposeMembersToGuest
            {
                Instance,
                Type,
                TypeAndNull,
                InstanceAndNull,
                Null,
                All
            }
            public ulong Size => 1024 * 1024;
            public uint ExpectedReturnValue;
            public string ExpectedOutput;
            public string GuestBinaryPath { get; }
            public byte[] Workload;
            public int[] Args;
            public Type? instanceOrTypeType { get; }
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

            public TestData(Type instanceOrTypeType, string guestBinaryFileName, byte[]? workload, int[] args, string expectedOutput = "Hello, World!!\n", uint? expectedReturnValue = null, ExposeMembersToGuest exposeMembers = ExposeMembersToGuest.All) : this(guestBinaryFileName, workload, args, expectedOutput, expectedReturnValue)
            {
                this.instanceOrTypeType = instanceOrTypeType;
                this.ExposeMembers = exposeMembers;
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
                        ExposeMembersToGuest.InstanceAndNull => new object?[] { GetInstance(instanceOrTypeType), null },
                        ExposeMembersToGuest.Null => new object?[] { null },
                        ExposeMembersToGuest.All => new object?[] { instanceOrTypeType, GetInstance(instanceOrTypeType), null },
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
            public Func<string, int>? GuestMethod = null;
            public int HostMethod(string msg)
            {
                return GuestMethod!($"Host Received: {msg} from Guest");
            }
        }

        [ExposeToGuest(false)]
        public class ExposeStaticMethodsUsingAttribute
        {
            public void GetNothing() { }

            public static void StaticGetNothing() { }
            [ExposeToGuest(true)]
            public static int StaticGetInt() { return 0; }
            public void GetNothingWithArgs(string arg1, int arg2) { }
            public static void StaticGetNothingWithArgs(string arg1, int arg2) { }
            public static Func<string, int>? GuestMethod = null;
            [ExposeToGuest(true)]
            public static int HostMethod(string msg)
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

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Loads_Windows_Exe(TestData testData)
        {
            SandboxRunOptions[] options = { SandboxRunOptions.RunFromGuestBinary, SandboxRunOptions.RunFromGuestBinary | SandboxRunOptions.RunInProcess };
            foreach (var option in options)
            {
                RunTests(testData, option, SimpleTest);
            }
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Loads_Windows_Exe_Concurrently(TestData testData)
        {
            SandboxRunOptions[] options = { SandboxRunOptions.RunFromGuestBinary, SandboxRunOptions.RunFromGuestBinary | SandboxRunOptions.RunInProcess };
            foreach (var option in options)
            {
                var output1 = new StringWriter();
                var sandbox1 = new Sandbox(testData.Size, testData.GuestBinaryPath, option, output1);
                SimpleTest(sandbox1, testData, output1, null);
                var output2 = new StringWriter();
                Sandbox sandbox2 = null;
                var ex = Record.Exception(() => sandbox2 = new Sandbox(testData.Size, testData.GuestBinaryPath, option, output2));
                Assert.NotNull(ex);
                Assert.IsType<System.ApplicationException>(ex);
                Assert.Equal("Only one instance of Sandbox is allowed when running from guest binary", ex.Message);
                sandbox1.Dispose();
                sandbox2?.Dispose();
                output1.Dispose();
                output2.Dispose();
                using (var output = new StringWriter())
                {
                    using (var sandbox = new Sandbox(testData.Size, testData.GuestBinaryPath, option, output))
                    {
                        SimpleTest(sandbox, testData, output);
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
                var binary = "simpleguest.exe";
                var path = AppDomain.CurrentDomain.BaseDirectory;
                var guestBinaryPath = Path.Combine(path, binary);
                ulong size = 1024 * 1024;
                var ex = Record.Exception(() => new Sandbox(size, guestBinaryPath, option));
                Assert.NotNull(ex);
                Assert.IsType<NotSupportedException>(ex);
                Assert.Equal("Cannot run in process on Linux", ex.Message);
            }
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Runs_InProcess(TestData testData)
        {
            SandboxRunOptions[] options = { SandboxRunOptions.RunInProcess, SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun };
            foreach (var option in options)
            {
                RunTests(testData, option, SimpleTest);
            }
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Runs_InProcess_Concurrently(TestData testData)
        {
            Parallel.For(0, numberOfParallelTests, (t) =>
            {
                SandboxRunOptions[] options = { SandboxRunOptions.RunInProcess, SandboxRunOptions.RunInProcess | SandboxRunOptions.RecycleAfterRun };
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

        [TheorySkipIfHyperVisorNotPresent]
        [MemberData(nameof(GetSimpleTestData))]
        public void Test_Runs_InHyperVisor_Concurrently(TestData testData)
        {

            Parallel.For(0, numberOfParallelTests, (t) =>
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
            SandboxRunOptions[] options = { SandboxRunOptions.RunFromGuestBinary, SandboxRunOptions.RunFromGuestBinary | SandboxRunOptions.RunInProcess };
            foreach (var option in options)
            {
                RunTests(testData, option, CallbackTest);
            }
        }

        [TheorySkipIfNotWindows]
        [MemberData(nameof(GetCallbackTestData))]
        public void Test_Loads_Windows_Exe_With_Callback_Concurrently(TestData testData)
        {
            SandboxRunOptions[] options = { SandboxRunOptions.RunFromGuestBinary, SandboxRunOptions.RunFromGuestBinary | SandboxRunOptions.RunInProcess };
            foreach (var option in options)
            {
                foreach (var instanceOrType in testData.TestInstanceOrTypes())
                {
                    var output1 = new StringWriter();
                    var sandbox1 = new Sandbox(testData.Size, testData.GuestBinaryPath, option, instanceOrType, output1);
                    CallbackTest(sandbox1, testData, output1, instanceOrType);
                    var output2 = new StringWriter();
                    Sandbox sandbox2 = null;
                    object instanceOrType1;
                    object instanceOrType2;
                    if (instanceOrType is not null && instanceOrType is not Type)
                    {
                        instanceOrType1 = GetInstance(instanceOrType.GetType());
                        instanceOrType2 = GetInstance(instanceOrType.GetType());
                    }
                    else
                    {
                        instanceOrType1 = instanceOrType;
                        instanceOrType2 = instanceOrType;
                    }
                    var ex = Record.Exception(() => sandbox2 = new Sandbox(testData.Size, testData.GuestBinaryPath, option, instanceOrType1, output2));
                    Assert.NotNull(ex);
                    Assert.IsType<System.ApplicationException>(ex);
                    Assert.Equal("Only one instance of Sandbox is allowed when running from guest binary", ex.Message);
                    sandbox1.Dispose();
                    sandbox2?.Dispose();
                    output1.Dispose();
                    output2.Dispose();
                    using (var output = new StringWriter())
                    {
                        using (var sandbox = new Sandbox(testData.Size, testData.GuestBinaryPath, option, instanceOrType2, output))
                        {
                            CallbackTest(sandbox, testData, output, instanceOrType2);
                        }
                    }
                }
            }
        }

        [Theory]
        [MemberData(nameof(GetCallbackTestData))]
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
        public void Test_Runs_InProcess_With_Callback_Concurrently(TestData testData)
        {
            Parallel.For(0, numberOfParallelTests, (t) =>
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
            Parallel.For(0, numberOfParallelTests, (t) =>
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

        private void RunTests(TestData testData, SandboxRunOptions options, Action<Sandbox, TestData, StringWriter, object> test)
        {
            if (testData.TestInstanceOrTypes().Length > 0)
            {
                foreach (var instanceOrType in testData.TestInstanceOrTypes())
                {
                    RunTest(testData, options, instanceOrType, test);
                }
            }
            else
            {
                RunTest(testData, options, test);
            }
        }

        private void RunTest(TestData testData, SandboxRunOptions sandboxRunOptions, object? instanceOrType, Action<Sandbox, TestData, StringWriter, object> test)
        {
            using (var output = new StringWriter())
            {
                using (var sandbox = new Sandbox(testData.Size, testData.GuestBinaryPath, sandboxRunOptions, instanceOrType, output))
                {
                    test(sandbox, testData, output, instanceOrType);
                }
            }
        }

        private void RunTest(TestData testData, SandboxRunOptions sandboxRunOptions, Action<Sandbox, TestData, StringWriter, object?> test)
        {
            using (var output = new StringWriter())
            {
                using (var sandbox = new Sandbox(testData.Size, testData.GuestBinaryPath, sandboxRunOptions, output))
                {
                    test(sandbox, testData, output, null);
                }
            }
        }

        private void RunTest(TestData testData, object? instanceOrType, Action<Sandbox, TestData, StringWriter, object?> test)
        {
            using (var output = new StringWriter())
            {
                using (var sandbox = instanceOrType == null ? new Sandbox(testData.Size, testData.GuestBinaryPath, output) : new Sandbox(testData.Size, testData.GuestBinaryPath, output, instanceOrType))
                {
                    test(sandbox, testData, output, instanceOrType);
                }
            }
        }

        private void RunTest(TestData testData, Action<Sandbox, TestData, StringWriter, object?> test)
        {
            using (var output = new StringWriter())
            {
                using (var sandbox = new Sandbox(testData.Size, testData.GuestBinaryPath, output))
                {
                    test(sandbox, testData, output, null);
                }
            }
        }

        private void SimpleTest(Sandbox sandbox, TestData testData, StringWriter output, object? notused = null)
        {
            (var result, _, _) = sandbox.Run(testData.Workload, testData.Args[0], testData.Args[1], testData.Args[2]);
            Assert.Equal<uint>(testData.ExpectedReturnValue, result);
            Assert.Equal(testData.ExpectedOutput, output.ToString());
        }

        private void CallbackTest(Sandbox sandbox, TestData testData, StringWriter output, object typeorinstance)
        {
            if (typeorinstance == null)
            {
                var ex = Record.Exception(() => sandbox.Run(testData.Workload, testData.Args[0], testData.Args[1], testData.Args[2]));
                Assert.NotNull(ex);
                Assert.IsType<ArgumentNullException>(ex);
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
                new object[] { new TestData(typeof(NoExposedMembers), "simpleguest.exe", null, new int[3] { 0, 0, 0 }) },
                new object[] { new TestData(typeof(ExposedMembers), "simpleguest.exe", null, new int[3] { 0, 0, 0 }) },
                new object[] { new TestData(typeof(ExposeStaticMethodsUsingAttribute), "simpleguest.exe", null, new int[3] { 0, 0, 0 }) },
                new object[] { new TestData(typeof(ExposeInstanceMethodsUsingAttribute), "simpleguest.exe", null, new int[3] { 0, 0, 0 }) },
                new object[] { new TestData(typeof(DontExposeSomeMembersUsingAttribute), "simpleguest.exe", null, new int[3] { 0, 0, 0 }) },
                new object[] { new TestData("simpleguest.exe", null, new int[3] { 0, 0, 0 }) } ,
            };
        }
        // This does not test passing null as instanceortype as currently this is not implmented properly in Sandbox and cause the testhost to crash.
        public static IEnumerable<object[]> GetCallbackTestData()
        {
            return new List<object[]>
            {
                new object[] { new TestData(typeof(ExposedMembers), "callbackguest.exe", null, new int[3] { 0, 0, 0 }, "Hello from GuestFunction, Host Received: Hello, World!! from Guest!!.\n", 70, TestData.ExposeMembersToGuest.Instance) },
                new object[] { new TestData(typeof(ExposeStaticMethodsUsingAttribute), "callbackguest.exe", null, new int[3] { 0, 0, 0 }, "", 14, TestData.ExposeMembersToGuest.Type) },
                new object[] { new TestData(typeof(ExposeInstanceMethodsUsingAttribute), "callbackguest.exe", null, new int[3] { 0, 0, 0 }, "Hello from GuestFunction, Host Received: Hello, World!! from Guest!!.\n", 70, TestData.ExposeMembersToGuest.Instance) },
            };
        }

        private static object GetInstance(Type instanceOrTypeType) => Activator.CreateInstance(instanceOrTypeType);

        static bool RunningOnWindows => RuntimeInformation.IsOSPlatform(OSPlatform.Windows);
    }
}
