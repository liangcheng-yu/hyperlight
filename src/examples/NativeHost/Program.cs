using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Threading.Tasks;
using Hyperlight;


namespace NativeHost
{
    class Program
    {
        private static readonly CancellationTokenSource cancellationTokenSource = new();
        private static readonly BlockingCollection<string> OutputBuffer = new();
        const int DEFAULT_NUMBER_OF_PARALLEL_INSTANCES = 10;
        const int DEFAULT_NUMBER_OF_ITERATIONS = 10;
        static void Main()
        {
            // check this is a supported platform.

            if (!Sandbox.IsSupportedPlatform)
            {
                Console.WriteLine($"{RuntimeInformation.OSDescription} is an unsupported platform!");
            }

            Console.WriteLine();

            var task = Task.Run(WriteToConsole);
            //TODO: Test and fix linux multi-instance and then remove this check
            var numberofparallelInstances = 1;
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                numberofparallelInstances = GetNumberOfParallelInstances();
            }

            var numberofIterations = GetNumberOfIterations();

            Console.WriteLine();

            // Get the path to a guest binary to run in HyperLight, this one just outputs "Hello World!"

            var path = AppDomain.CurrentDomain.BaseDirectory;
            var guestBinary = "simpleguest.exe";
            var guestBinaryPath = Path.Combine(path, guestBinary);
            var size = (ulong)1048576; // 1MB

            // Hyperlight enables binaries to be run by loading an exe into memory using LoadLibrary, this is useful for debugging
            // It only works on windows, and only one concurrent instance is supported.

            var options = SandboxRunOptions.RunFromGuestBinary;

            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                Console.WriteLine($"Running guest binary {guestBinary} by loading exe into memory");
                Console.WriteLine();

                using (var inProcSandboxWithCallBack = new Sandbox(size, guestBinaryPath, options))
                {
                    (var returnValue, var _, var _) = inProcSandboxWithCallBack.Run(null, 0, 0, 0);
                    Console.WriteLine($"Guest returned {returnValue}");
                }

                WaitForUserInput();

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);

                Console.WriteLine($"Running guest binary {guestBinary} by loading exe into memory with host and guest method execution");
                Console.WriteLine();

                using (var inProcSandboxWithCallBack = new Sandbox(size, guestBinaryPath, options, new ExposedMethods()))
                {
                    (var returnValue, var _, var _) = inProcSandboxWithCallBack.Run(null, 0, 0, 0);
                    Console.WriteLine($"Guest returned {returnValue}");
                }

                WaitForUserInput();
            }

            // The binary can also be executed by copying it into host process memory and executing.

            guestBinary = "simpleguest.exe";
            guestBinaryPath = Path.Combine(path, guestBinary);
            options = SandboxRunOptions.RunInProcess;

            Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} from memory");
            Console.WriteLine();
            var stopWatch = Stopwatch.StartNew();
            Parallel.For(0, numberofparallelInstances, i =>
            {
                using (var writer = new StringWriter())
                {
                    using (var sandbox = new Sandbox(size, guestBinaryPath, options, writer))
                    {
                        (var returnValue, var _, var _) = sandbox.Run(null, 0, 0, 0);
                        OutputBuffer.Add($"Instance {i}:{writer}");
                        OutputBuffer.Add($"Instance {i}:Guest returned {returnValue}");
                    }
                }
            });

            stopWatch.Stop();
            var elapsed = stopWatch.Elapsed;

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
                using (var writer = new StringWriter())
                {
                    using (var sandbox = new Sandbox(size, guestBinaryPath, options, new ExposedMethods(), writer))
                    {
                        (var returnValue, var _, var _) = sandbox.Run(null, 0, 0, 0);
                        OutputBuffer.Add($"Instance {i}:Guest returned {returnValue}");
                    }
                }
            });

            stopWatch.Stop();
            elapsed = stopWatch.Elapsed;

            Console.WriteLine();
            Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{elapsed.Ticks / 10 % 1000:000} seconds");


            WaitForUserInput();

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
                        using (var sandbox = new Sandbox(size, guestBinaryPath, writer))
                        {
                            sandbox.Run(null, 0, 0, 0);
                            OutputBuffer.Add($"Instance {i}:{writer}");
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{elapsed.Ticks / 10 % 1000:000} seconds");

                WaitForUserInput();

                // The guest binary can invoke methods exported by the host process.
                // The host process can also invoke methods exported by the guest binary.
                // Host methods to be exposed can be static or instance methods on a type or an object.
                // Attributes can be used to control which methods are exposed.
                // Guest methods are represented by delegates on the type.
                // The type or object containing the methods/delegates is passed to the sandbox as a parameter

                // Call between the guest and host in-process.

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} in Hypervisor sandbox with host and guest method execution");
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();

                Parallel.For(0, numberofparallelInstances, i =>
                {
                    using (var writer = new StringWriter())
                    {
                        using (var sandbox = new Sandbox(size, guestBinaryPath, options, new ExposedMethods(), writer))
                        {
                            (var returnValue, var _, var _) = sandbox.Run(null, 0, 0, 0);
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
                options = SandboxRunOptions.RecycleAfterRun;

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} in Hypervisor sandbox {numberofIterations} times");
                Console.WriteLine();
                Console.WriteLine();
                stopWatch = Stopwatch.StartNew();

                Parallel.For(0, numberofparallelInstances, p =>
                {
                    using (var writer = new StringWriter())
                    {
                        using (var hypervisorSandbox = new Sandbox(size, guestBinaryPath, options, writer))
                        {
                            var builder = writer.GetStringBuilder();
                            for (var i = 0; i < numberofIterations; i++)
                            {
                                hypervisorSandbox.Run(null, 0, 0, 0);
                                OutputBuffer.Add($"Instance {p} Iteration {i}:{builder.ToString()}");
                                builder.Remove(0, builder.Length);
                            }
                        }
                    }
                });

                stopWatch.Stop();
                elapsed = stopWatch.Elapsed;

                Console.WriteLine();
                Console.WriteLine($"Created and run {numberofparallelInstances} parallel instances with {numberofIterations} iterations in {elapsed.TotalSeconds:00}.{elapsed.Milliseconds:000}{elapsed.Ticks / 10 % 1000:000} seconds");

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
                        using (var hypervisorSandbox = new Sandbox(size, guestBinaryPath, options, new ExposedMethods(), writer))
                        {
                            var builder = writer.GetStringBuilder();
                            for (var i = 0; i < numberofIterations; i++)
                            {
                                (var returnValue, var _, var _) = hypervisorSandbox.Run(null, 0, 0, 0);
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
            Console.WriteLine();
            Console.WriteLine("Press any key to continue...");
            Console.WriteLine();
            if (!Debugger.IsAttached)
            {
                Console.ReadKey();
            }
        }

        static int GetNumberOfParallelInstances()
        {
            var numberOfInstances = DEFAULT_NUMBER_OF_PARALLEL_INSTANCES;
            Console.WriteLine();
            Console.WriteLine($"Enter the number of Sandbox Instances to concurrently create and press enter (default: {DEFAULT_NUMBER_OF_PARALLEL_INSTANCES})...");
            Console.WriteLine();
            if (!Debugger.IsAttached)
            {
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
            Console.WriteLine();
            Console.WriteLine($"Enter the number of iterations to execute in a recycled sandbox instance (default: {DEFAULT_NUMBER_OF_ITERATIONS})...");
            Console.WriteLine();
            if (!Debugger.IsAttached)
            {
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
