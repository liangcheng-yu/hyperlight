using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using Hyperlight;
using Hyperlight.Core;
using HyperlightDependencies;

namespace NativeHost
{
    class Program
    {
        private static readonly CancellationTokenSource cancellationTokenSource = new();
        private static readonly BlockingCollection<string> OutputBuffer = new();

        private static bool waitforuserinput = true;
        const int DEFAULT_NUMBER_OF_PARALLEL_INSTANCES = 100;
        const int DEFAULT_NUMBER_OF_ITERATIONS = 50;
        static void Main()
        {
            using var ctx = new Hyperlight.Wrapper.Context();
            var sandboxMemoryConfig = new SandboxMemoryConfiguration(ctx);
            foreach (var arg in Environment.GetCommandLineArgs())
            {
                if (arg.ToLowerInvariant().Contains("nowait"))
                {
                    waitforuserinput = false;
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

            var task = Task.Run(WriteToConsole);

            var numberofparallelInstances = 1;
            numberofparallelInstances = GetNumberOfParallelInstances();

            var numberofIterations = GetNumberOfIterations();

            Console.WriteLine();

            // Get the path to a guest binary to run in HyperLight, this one just outputs "Hello World!"

            var path = AppDomain.CurrentDomain.BaseDirectory;
            var guestBinary = "simpleguest.exe";
            var guestBinaryPath = Path.Combine(path, guestBinary);
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

                Console.WriteLine($"Running guest binary {guestBinary} by loading exe into memory");
                Console.WriteLine();

                using (var sandbox = new Sandbox(
                    sandboxMemoryConfig,
                    guestBinaryPath,
                    options
                ))
                {
                    var guestMethods = new ExposedMethods();
                    sandbox.BindGuestFunction("PrintOutput", guestMethods);
                    var returnValue = guestMethods.PrintOutput!("Hello World!!!!!");
                    Console.WriteLine();
                    Console.WriteLine($"Guest returned {returnValue}");
                }

                WaitForUserInput();

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);

                Console.WriteLine($"Running guest binary {guestBinary} by loading exe into memory with host and guest method execution");
                Console.WriteLine();
                var exposedMethods = new ExposedMethods();
                using (var sandbox = new Sandbox(
                    sandboxMemoryConfig,
                    guestBinaryPath,
                    options
                ))
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

                Console.WriteLine($"Running guest binary {guestBinary} by loading exe into memory and customising Sandbox initialisation");
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

                using (var sandbox = new Sandbox(
                    sandboxMemoryConfig,
                    guestBinaryPath,
                    options,
                    func
                ))
                {
                    var returnValue = exposedMethods.PrintOutput!("Hello World!!!!!");
                    Console.WriteLine();
                    Console.WriteLine($"Guest returned {returnValue}");
                }

                WaitForUserInput();

                // The binary can also be executed by copying it into host process memory and executing.

                guestBinary = "simpleguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);
                options = SandboxRunOptions.RunInProcess;

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} from memory");
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();
                Parallel.For(0, numberofparallelInstances, i =>
                {
                    using (var writer = new StringWriter())
                    {
                        var sboxBuilder = new SandboxBuilder()
                            .WithConfig(sandboxMemoryConfig)
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithRunOptions(options)
                            .WithWriter(writer);
                        using (var sandbox = sboxBuilder.Build())
                        {
                            var guestMethods = new ExposedMethods();
                            sandbox.BindGuestFunction("PrintOutput", guestMethods);
                            var returnValue = guestMethods.PrintOutput!($"Hello World!! from Instance {i}");
                            OutputBuffer.Add($"Instance {i}:{writer}");
                            OutputBuffer.Add($"Instance {i}:Guest returned {returnValue}");
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{elapsed.Ticks / 10 % 1000:000} seconds");

                WaitForUserInput();

                // Call between the guest and host in-process.

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} from memory with host and guest method execution");
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();
                Parallel.For(0, numberofparallelInstances, i =>
                {
                    OutputBuffer.Add($"Instance {i}:");
                    using (var writer = new StringWriter())
                    {
                        OutputBuffer.Add($"Created Writer Instance {i}:");
                        var exposedMethods = new ExposedMethods();
                        var sboxBuilder = new SandboxBuilder()
                            .WithConfig(sandboxMemoryConfig)
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithRunOptions(options)
                            .WithWriter(writer);
                        using (var sandbox = sboxBuilder.Build())
                        {
                            sandbox.ExposeAndBindMembers(exposedMethods);
                            OutputBuffer.Add($"Created Sandbox Instance {i}:");
                            var returnValue = sandbox.CallGuest<int>(() => { return exposedMethods.GuestMethod!($"Hello from Hyperlight Host: Instance{i}"); });
                            OutputBuffer.Add($"Instance {i}:{writer}");
                            OutputBuffer.Add($"Instance {i}:Guest returned {returnValue}");
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{elapsed.Ticks / 10 % 1000:000} seconds");

                WaitForUserInput();
            }

            guestBinary = "simpleguest.exe";
            guestBinaryPath = Path.Combine(path, guestBinary);

            // if a supported Hypervisor is available, we can run the guest binary in a Hypervisor sandbox.

            if (Sandbox.IsHypervisorPresent())
            {

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} in Hypervisor sandbox");
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();

                Parallel.For(0, numberofparallelInstances, i =>
                {
                    using (var writer = new StringWriter())
                    {
                        var sboxBuilder = new SandboxBuilder()
                            .WithConfig(sandboxMemoryConfig)
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithWriter(writer);
                        using (var sandbox = sboxBuilder.Build())
                        {
                            var guestMethods = new ExposedMethods();
                            sandbox.BindGuestFunction("PrintOutput", guestMethods);
                            var returnValue = guestMethods.PrintOutput!($"Hello World from instance {i}!!!!!");
                            OutputBuffer.Add($"Instance {i}:{writer}");
                            OutputBuffer.Add($"Instance {i}:Guest returned {returnValue}");
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{elapsed.Ticks / 10 % 1000:000} seconds");

                WaitForUserInput();

                // Call between the guest and host .

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} in Hypervisor sandbox with host and guest method execution");
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();

                Parallel.For(0, numberofparallelInstances, i =>
                {
                    using (var writer = new StringWriter())
                    {
                        var exposedMethods = new ExposedMethods();
                        var sboxBuilder = new SandboxBuilder()
                            .WithConfig(sandboxMemoryConfig)
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithWriter(writer);
                        using (var sandbox = sboxBuilder.Build())
                        {
                            sandbox.ExposeAndBindMembers(exposedMethods);
                            OutputBuffer.Add($"Created Sandbox Instance {i}:");
                            var returnValue = sandbox.CallGuest<int>(() => { return exposedMethods.GuestMethod!($"Hello Guest in Hypervisor from Hyperlight Host: Instance{i}"); });
                            OutputBuffer.Add($"Instance {i}:{writer}");
                            OutputBuffer.Add($"Instance {i}:Guest returned {returnValue}");
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{elapsed.Ticks / 10 % 1000:000} seconds");

                guestBinary = "simpleguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);

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
                        using (var hypervisorSandbox = new Sandbox(
                            sandboxMemoryConfig,
                            guestBinaryPath,
                            options,
                            func,
                            writer
                        ))
                        {
                            OutputBuffer.Add($"Instance {p} Initialisation:{builder.ToString()}");
                            builder.Remove(0, builder.Length);
                            for (var i = 0; i < numberofIterations; i++)
                            {
                                var returnValue = exposedMethods.PrintOutput!("Hello World!!!!!");
                                OutputBuffer.Add($"Instance {p} Iteration {i}:{builder.ToString()}");
                                builder.Remove(0, builder.Length);
                            }
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances with {numberofIterations} iterations and custom Sandbox initialisation in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{elapsed.Ticks / 10 % 1000:000} seconds");

                WaitForUserInput();

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} in Hypervisor sandbox with host and guest method execution {numberofIterations} times");
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();

                Parallel.For(0, numberofparallelInstances, p =>
                {
                    using (var writer = new StringWriter())
                    {
                        var exposedMethods = new ExposedMethods();
                        var sboxBuilder = new SandboxBuilder()
                            .WithConfig(sandboxMemoryConfig)
                            .WithGuestBinaryPath(guestBinaryPath)
                            .WithRunOptions(options)
                            .WithWriter(writer);
                        using (var hypervisorSandbox = sboxBuilder.Build())
                        {
                            hypervisorSandbox.ExposeAndBindMembers(exposedMethods);
                            var builder = writer.GetStringBuilder();
                            for (var i = 0; i < numberofIterations; i++)
                            {
                                var returnValue = hypervisorSandbox.CallGuest<int>(() => { return exposedMethods.GuestMethod!($"Hello Guest in Hypervisor from Hyperlight Host: Instance{p} Iteration {i}"); });
                                OutputBuffer.Add($"Instance {p} Iteration {i}:{builder.ToString()}");
                                builder.Remove(0, builder.Length);
                                OutputBuffer.Add($"Instance {p} Iteration {i}:Guest returned {returnValue}");
                            }
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances with {numberofIterations} iterations in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{elapsed.Ticks / 10 % 1000:000} seconds");

            }
            Console.WriteLine("Done");
            cancellationTokenSource.Cancel();
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
            while (true)
            {
                var message = OutputBuffer.Take(cancellationTokenSource.Token);
                Console.WriteLine(message);
            }
        }
    }
}
