using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading.Tasks;
using Hyperlight;

namespace NativeHost
{
    class Program
    {
        const int DEFAULT_NUMBER_OF_PARALLEL_INSTANCES = 1;
        static void Main()
        {
            var numberofparallelInstances = GetNumberOfParallelInstances();
            var sandbox = new Sandbox?[numberofparallelInstances];
            // check this is a supported platform.

            if (!Sandbox.IsSupportedPlatform)
            {
                Console.WriteLine($"{RuntimeInformation.OSDescription} is an unsupported platform!");
            }

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

                using (var inProcSandboxWithCallBack = new Sandbox(size, guestBinaryPath, options))
                {
                    (var returnValue, var _, var _) = inProcSandboxWithCallBack.Run(null, 0, 0, 0);
                    Console.WriteLine($"Guest returned {returnValue}");
                }

                WaitForUserInput();

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);

                Console.WriteLine($"Running guest binary {guestBinary} by loading exe into memory with host and guest method execution");

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


            Parallel.For(0, numberofparallelInstances, (i) =>
            {
                using (var sandbox = new Sandbox(size, guestBinaryPath, options))
                {
                    (var returnValue, var _, var _) = sandbox.Run(null, 0, 0, 0);
                    Console.WriteLine($"Guest returned {returnValue}");
                }
            });

            WaitForUserInput();

            //TODO: fix this on Linux

            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
               // Call between the guest and host in-process.

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} from memory with host and guest method execution");

                Parallel.For(0, numberofparallelInstances, (i) =>
                {
                    using (var sandbox = new Sandbox(size, guestBinaryPath, options, new ExposedMethods()))
                    {
                        (var returnValue, var _, var _) = sandbox.Run(null, 0, 0, 0);
                        Console.WriteLine($"Guest returned {returnValue}");
                    }
                });

                WaitForUserInput();
            }

            guestBinary = "simpleguest.exe";
            guestBinaryPath = Path.Combine(path, guestBinary);

            // if a supported Hypervisor is available, we can run the guest binary in a Hypervisor sandbox.

            if (Sandbox.IsHypervisorPresent())
            {

                //TODO get rid of this once we figre out the issue with creating multiple WHP Partitions

                if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
                {
                    numberofparallelInstances = 1;
                }

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} in Hypervisor sandbox");


                Parallel.For(0, numberofparallelInstances, (i) =>
                {
                    using (var hypervisorSandbox = new Sandbox(size, guestBinaryPath))
                    {
                        hypervisorSandbox.Run(null, 0, 0, 0);
                    }
                });

                WaitForUserInput();

                // The guest binary can invoke methods exported by the host process.
                // The host process can also invoke methods exported by the guest binary.
                // Host methods to be exposed can be static or instance methods on a type or an object.
                // Attributes can be used to control which methods are exposed.
                // Guest methods are represented by delegates on the type.
                // The type or object containing the methods/delegates is passed to the sandbox as a parameter

                // Call between the guest and host in-process.

                // TODO: Fix this on Linux.

                if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
                {
                    guestBinary = "callbackguest.exe";
                    guestBinaryPath = Path.Combine(path, guestBinary);

                    Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} in Hypervisor sandbox with host and guest method execution");

                    Parallel.For(0, numberofparallelInstances, (i) =>
                    {
                        using (var sandbox = new Sandbox(size, guestBinaryPath, options, new ExposedMethods()))
                        {
                            (var returnValue, var _, var _) = sandbox.Run(null, 0, 0, 0);
                            Console.WriteLine($"Guest returned {returnValue}");
                        }
                    });
                }

                guestBinary = "simpleguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);
                options = SandboxRunOptions.RecycleAfterRun;

                Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} in Hypervisor sandbox 5 times");

                Parallel.For(0, numberofparallelInstances, (_) =>
                {
                    using (var hypervisorSandbox = new Sandbox(size, guestBinaryPath, options))
                    {
                        for (var i = 0; i < 5; i++)
                        {
                            hypervisorSandbox.Run(null, 0, 0, 0);
                        }
                    }
                });

                // TODO: Fix this on Linux.

                if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
                {

                    WaitForUserInput();

                    guestBinary = "callbackguest.exe";
                    guestBinaryPath = Path.Combine(path, guestBinary);

                    Console.WriteLine($"Running {numberofparallelInstances} parallel instances of guest binary {guestBinary} in Hypervisor sandbox with host and guest method execution 5 times");

                    Parallel.For(0, numberofparallelInstances, (_) =>
                    {
                        using (var hypervisorSandbox = new Sandbox(size, guestBinaryPath, options, new ExposedMethods()))
                        {
                            for (var i = 0; i < 5; i++)
                            {
                                (var returnValue, var _, var _) = hypervisorSandbox.Run(null, 0, 0, 0);
                                Console.WriteLine($"Guest returned {returnValue}");
                            }
                        }
                    });
                }
            }
            Console.WriteLine("Done");
        }

        static void WaitForUserInput()
        {
            Console.WriteLine("Press any key to continue...");
            if (!Debugger.IsAttached)
            {
                Console.ReadKey();
            }
        }

        static int GetNumberOfParallelInstances()
        {
            var numberOfInstances = DEFAULT_NUMBER_OF_PARALLEL_INSTANCES;
            Console.WriteLine($"Enter the number of Sandbox Instances to concurrently create and press enter (default: {DEFAULT_NUMBER_OF_PARALLEL_INSTANCES})...");
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
    }
}
