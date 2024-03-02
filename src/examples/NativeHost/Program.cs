using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using Hyperlight;
using Hyperlight.Core;
using HyperlightDependencies;

namespace NativeHost
{
    class Program
    {
        private static readonly BlockingCollection<string> outputBuffer = new();
        private static bool waitforuserinput = true;
        private static string guestType = "rust";
        const int DEFAULT_NUMBER_OF_PARALLEL_INSTANCES = 100;
        const int DEFAULT_NUMBER_OF_ITERATIONS = 50;
        static void Main()
        {
            foreach (var arg in Environment.GetCommandLineArgs())
            {
                if (arg.ToLowerInvariant().Contains("nowait"))
                {
                    waitforuserinput = false;
                }

                if (arg.ToLowerInvariant().Contains("usecguests"))
                {
                    guestType = "c";
                }
            }

            // check this is a supported platform.
            if (!Sandbox.IsSupportedPlatform)
            {
                Console.WriteLine($"{RuntimeInformation.OSDescription} is an unsupported platform!");
                Environment.Exit(1);
            }

            if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows) && !Sandbox.IsHypervisorPresent())
            {
                Console.WriteLine($"Hyperlight is not supported on {RuntimeInformation.OSDescription}!");
            }

            Console.WriteLine();

            var writeToConsoleTask = Task.Run(WriteToConsole);

            var numberofparallelInstances = GetNumberOfParallelInstances();

            var numberofIterations = GetNumberOfIterations();

            Console.WriteLine();

            // Get the path to a guest binary to run in HyperLight, this one just outputs "Hello World!"

            var path = AppDomain.CurrentDomain.BaseDirectory;
            var guestBinary = "simpleguest.exe";
            var guestBinaryPath = Path.Combine(path, "guests", guestType, guestBinary);
            Stopwatch stopWatch;
            TimeSpan elapsed;

            // Hyperlight enables binaries to be run by loading an exe into memory using LoadLibrary, this is useful for debugging
            // It only works on windows, and only one concurrent instance is supported.

            // It is also possible to load a binary into memory and run it , at the moment this only works reliably on windows. (the host is a PE file and dotnet does not support fastcall convention so default linux calling convetion cannot be overridden)

            var options = SandboxRunOptions.RunFromGuestBinary;

            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                // The host process can invoke methods exported by the guest binary.
                // The guest binary can also invoke methods exported by the host process.
                // Host methods to be exposed can be static or instance methods on a type or an object.
                // Guest methods are represented by delegates .
                // Attributes can be used to control which methods/delegates are exposed.
                // The type or object containing the methods/delegates can be passed to the sandbox as a constructor parameter
                // Methods or delegates can also be exposed or bound to the guest explicitly using the BindGuestFunction or ExposeHostMethod methods.

                Console.WriteLine($"Running {guestType} guest binary {guestBinaryPath} by loading exe into memory");
                Console.WriteLine();

                var config = new SandboxConfiguration();

                SandboxBuilder builder = new SandboxBuilder()
                    .WithGuestBinaryPath(guestBinaryPath)
                    .WithRunOptions(options)
                    .WithConfig(config);

                using (var sandbox = builder.Build())
                {
                    var guestMethods = new ExposedMethods();
                    sandbox.BindGuestFunction("PrintOutput", guestMethods);
                    var returnValue = guestMethods.PrintOutput!("Hello World!!!!!");
                    Console.WriteLine();
                    Console.WriteLine($"Guest returned {returnValue}");
                }

                WaitForUserInput();

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, "guests", guestType, guestBinary);

                Console.WriteLine($"Running {guestType} guest binary {guestBinary} by loading exe into memory with host and guest method execution");
                Console.WriteLine();
                var exposedMethods = new ExposedMethods();
                builder = new SandboxBuilder()
                    .WithGuestBinaryPath(guestBinaryPath)
                    .WithRunOptions(options)
                    .WithConfig(config);

                using (var sandbox = builder.Build())
                {
                    sandbox.ExposeAndBindMembers(exposedMethods);

                    // The GuestMethod Delegate has to be called in the context of the Sandboxes CallGuest method as the GuestMethod calls back into the host which in turn calls back to the PrintOutput function
                    // without executing the call via CallGuest the state of the sandbox would be reset between the GuestMethod and PrintOutput calls and the call would fail.

                    var returnValue = sandbox.CallGuest<int>(() => { return exposedMethods.GuestMethod!("Hello from Hyperlight Host"); });
                    Console.WriteLine();
                    Console.WriteLine($"Guest returned {returnValue}");
                }

                WaitForUserInput();


                // The initialisation of the Sandbox is performed by invoking the entrypoint on the GuestBinary
                // To enable customisation of the initialisation functions in the guest can be called during initialistation 
                // This is done by passing an Action<Sandbox> to the Sandbox constructor, the constructor will call the Action 
                // after the entrypoint in the guest binary has been called and before the state of the Sandbox is snapshotted

                Console.WriteLine($"Running {guestType} guest binary {guestBinary} by loading exe into memory and customising Sandbox initialisation");
                Console.WriteLine();

                exposedMethods = new ExposedMethods();
                Action<ISandboxRegistration> func = (s) =>
                {
                    s.BindGuestFunction("PrintOutput", exposedMethods);
                    Console.WriteLine();
                    exposedMethods.PrintOutput!("Hello from Sandbox Initialisation");
                    Console.WriteLine();
                    exposedMethods.PrintOutput!("Hello again from Sandbox Initialisation");
                    Console.WriteLine();
                };

                builder = new SandboxBuilder()
                    .WithGuestBinaryPath(guestBinaryPath)
                    .WithRunOptions(options)
                    .WithInitFunction(func)
                    .WithConfig(config);

                using (var sandbox = builder.Build())
                {
                    var returnValue = exposedMethods.PrintOutput!("Hello World!!!!!");
                    Console.WriteLine();
                    Console.WriteLine($"Guest returned {returnValue}");
                }

                WaitForUserInput();

                // The binary can also be executed by copying it into host process memory and executing.

                guestBinary = "simpleguest.exe";
                guestBinaryPath = Path.Combine(path, "guests", guestType, guestBinary);
                options = SandboxRunOptions.RunInProcess;

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of {guestType} guest binary {guestBinary} from memory");
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();
                Parallel.For(0, numberofparallelInstances, i =>
                {
                    using (var writer = new StringWriter())
                    {
                        var sboxBuilder = new SandboxBuilder()
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithRunOptions(options)
                            .WithWriter(writer)
                            .WithConfig(config);
                        using (var sandbox = sboxBuilder.Build())
                        {
                            var guestMethods = new ExposedMethods();
                            sandbox.BindGuestFunction("PrintOutput", guestMethods);
                            var returnValue = guestMethods.PrintOutput!($"Hello World!! from Instance {i}");
                            outputBuffer.Add($"Instance {i}:{writer}");
                            outputBuffer.Add($"Instance {i}:Guest returned {returnValue}");
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;
                var microseconds = 1000000000L / Stopwatch.Frequency * elapsed.Ticks % 1000000;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{microseconds:000} seconds");

                WaitForUserInput();

                // Call between the guest and host in-process.

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, "guests", guestType, guestBinary);

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of {guestType} guest binary {guestBinary} from memory with host and guest method execution");
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();
                Parallel.For(0, numberofparallelInstances, i =>
                {
                    outputBuffer.Add($"Instance {i}:");
                    using (var writer = new StringWriter())
                    {
                        outputBuffer.Add($"Created Writer Instance {i}:");
                        var exposedMethods = new ExposedMethods();
                        var sboxBuilder = new SandboxBuilder()
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithRunOptions(options)
                            .WithWriter(writer)
                            .WithConfig(config);
                        using (var sandbox = sboxBuilder.Build())
                        {
                            sandbox.ExposeAndBindMembers(exposedMethods);
                            outputBuffer.Add($"Created Sandbox Instance {i}:");
                            var returnValue = sandbox.CallGuest<int>(() => { return exposedMethods.GuestMethod!($"Hello from Hyperlight Host: Instance{i}"); });
                            outputBuffer.Add($"Instance {i}:{writer}");
                            outputBuffer.Add($"Instance {i}:Guest returned {returnValue}");
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;
                microseconds = 1000000000L / Stopwatch.Frequency * elapsed.Ticks % 1000000;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{microseconds:000} seconds");

                WaitForUserInput();
            }

            guestBinary = "simpleguest.exe";
            guestBinaryPath = Path.Combine(path,"guests", guestType, guestBinary);

            // if a supported Hypervisor is available, we can run the guest binary in a Hypervisor sandbox.

            if (Sandbox.IsHypervisorPresent())
            {

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of {guestType} guest binary {guestBinary} in Hypervisor sandbox");
                Console.WriteLine();

                var config = new SandboxConfiguration();

                stopWatch = Stopwatch.StartNew();

                Parallel.For(0, numberofparallelInstances, i =>
                {
                    using (var writer = new StringWriter())
                    {
                        var sboxBuilder = new SandboxBuilder()
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithWriter(writer)
                            .WithConfig(config);
                        using (var sandbox = sboxBuilder.Build())
                        {
                            var guestMethods = new ExposedMethods();
                            sandbox.BindGuestFunction("PrintOutput", guestMethods);
                            var returnValue = guestMethods.PrintOutput!($"Hello World from instance {i}!!!!!");
                            outputBuffer.Add($"Instance {i}:{writer}");
                            outputBuffer.Add($"Instance {i}:Guest returned {returnValue}");
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;
                var microseconds = 1000000000L / Stopwatch.Frequency * elapsed.Ticks % 1000000;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{microseconds:000} seconds");

                WaitForUserInput();

                // Call between the guest and host .

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, "guests", guestType, guestBinary);

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of {guestType} guest binary {guestBinary} in Hypervisor sandbox with host and guest method execution");
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();

                Parallel.For(0, numberofparallelInstances, i =>
                {
                    using (var writer = new StringWriter())
                    {
                        var exposedMethods = new ExposedMethods();
                        var sboxBuilder = new SandboxBuilder()
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithWriter(writer)
                            .WithConfig(config);
                        using (var sandbox = sboxBuilder.Build())
                        {
                            sandbox.ExposeAndBindMembers(exposedMethods);
                            outputBuffer.Add($"Created Sandbox Instance {i}:");
                            var returnValue = sandbox.CallGuest<int>(() => { return exposedMethods.GuestMethod!($"Hello Guest in Hypervisor from Hyperlight Host: Instance{i}"); });
                            outputBuffer.Add($"Instance {i}:{writer}");
                            outputBuffer.Add($"Instance {i}:Guest returned {returnValue}");
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;
                microseconds = 1000000000L / Stopwatch.Frequency * elapsed.Ticks % 1000000;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{microseconds:000} seconds");

                guestBinary = "simpleguest.exe";
                guestBinaryPath = Path.Combine(path, "guests", guestType, guestBinary);

                WaitForUserInput();

                options = SandboxRunOptions.RecycleAfterRun;
                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} in Hypervisor sandbox {numberofIterations} times with custom Sandbox initialisation.");
                Console.WriteLine();
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();

                Parallel.For(0, numberofparallelInstances, p =>
                {
                    using (var writer = new StringWriter())
                    {
                        var exposedMethods = new ExposedMethods();
                        Action<ISandboxRegistration> func = (s) =>
                        {
                            s.BindGuestFunction("PrintOutput", exposedMethods);
                            exposedMethods.PrintOutput!($"{Environment.NewLine}");
                            exposedMethods.PrintOutput!($"Hello from Sandbox Initialisation. {Environment.NewLine}");
                            exposedMethods.PrintOutput!($"Hello again from Sandbox Initialisation.{Environment.NewLine}");
                        };

                        var builder = writer.GetStringBuilder();
                        var sandboxBuilder = new SandboxBuilder()
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithRunOptions(options)
                            .WithInitFunction(func)
                            .WithWriter(writer)
                            .WithConfig(config);

                        using (var sandbox = sandboxBuilder.Build())
                        {
                            outputBuffer.Add($"Instance {p} Initialisation:{builder.ToString()}");
                            builder.Remove(0, builder.Length);
                            for (var i = 0; i < numberofIterations; i++)
                            {
                                var returnValue = exposedMethods.PrintOutput!("Hello World!!!!!");
                                outputBuffer.Add($"Instance {p} Iteration {i}:{builder.ToString()}");
                                builder.Remove(0, builder.Length);
                            }
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;
                microseconds = 1000000000L / Stopwatch.Frequency * elapsed.Ticks % 1000000;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances with {numberofIterations} iterations and custom Sandbox initialisation in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{microseconds:000} seconds");

                WaitForUserInput();

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, "guests", guestType, guestBinary);

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of {guestType} guest binary {guestBinary} in Hypervisor sandbox with host and guest method execution {numberofIterations} times");
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();

                Parallel.For(0, numberofparallelInstances, p =>
                {
                    using (var writer = new StringWriter())
                    {
                        var exposedMethods = new ExposedMethods();
                        var sboxBuilder = new SandboxBuilder()
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithRunOptions(options)
                            .WithWriter(writer)
                            .WithConfig(config);
                        using (var hypervisorSandbox = sboxBuilder.Build())
                        {
                            hypervisorSandbox.ExposeAndBindMembers(exposedMethods);
                            var builder = writer.GetStringBuilder();
                            for (var i = 0; i < numberofIterations; i++)
                            {
                                var returnValue = hypervisorSandbox.CallGuest<int>(() => { return exposedMethods.GuestMethod!($"Hello Guest in Hypervisor from Hyperlight Host: Instance{p} Iteration {i}"); });
                                outputBuffer.Add($"Instance {p} Iteration {i}:{builder.ToString()}");
                                builder.Remove(0, builder.Length);
                                outputBuffer.Add($"Instance {p} Iteration {i}:Guest returned {returnValue}");
                            }
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;
                microseconds = 1000000000L / Stopwatch.Frequency * elapsed.Ticks % 1000000;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances with {numberofIterations} iterations in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{microseconds:000} seconds");

            }

            outputBuffer.CompleteAdding();
            writeToConsoleTask.Wait();
            Console.WriteLine("Done");
        }

        static void WaitForUserInput()
        {
            if (!Debugger.IsAttached && waitforuserinput)
            {
                Console.WriteLine();
                Console.WriteLine("Press any key to continue...");
                Console.WriteLine();
                Console.ReadKey();
            }
        }

        static int GetNumberOfParallelInstances()
        {
            var numberOfInstances = DEFAULT_NUMBER_OF_PARALLEL_INSTANCES;

            if (!Debugger.IsAttached && waitforuserinput)
            {
                Console.WriteLine();
                Console.WriteLine($"Enter the number of Sandbox Instances to concurrently create and press enter (default: {DEFAULT_NUMBER_OF_PARALLEL_INSTANCES})...");
                Console.WriteLine();
                var input = Console.ReadLine();
                if (!int.TryParse(input, out numberOfInstances))
                {
                    numberOfInstances = DEFAULT_NUMBER_OF_PARALLEL_INSTANCES;
                }
            }
            return numberOfInstances;
        }

        static int GetNumberOfIterations()
        {
            var numberOfIterations = DEFAULT_NUMBER_OF_ITERATIONS;

            if (!Debugger.IsAttached && waitforuserinput)
            {
                Console.WriteLine();
                Console.WriteLine($"Enter the number of iterations to execute in a recycled sandbox instance (default: {DEFAULT_NUMBER_OF_ITERATIONS})...");
                Console.WriteLine();
                var input = Console.ReadLine();
                if (!int.TryParse(input, out numberOfIterations))
                {
                    numberOfIterations = DEFAULT_NUMBER_OF_ITERATIONS;
                }
            }
            return numberOfIterations;
        }

        static void WriteToConsole()
        {
            while (!outputBuffer.IsCompleted)
            {
                if (outputBuffer.TryTake(out var message, 100))
                {
                    Console.WriteLine(message);
                }
            }
        }
    }
}
