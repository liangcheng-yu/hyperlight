using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using Hyperlight;

namespace NativeHost
{
    class Program
    {
        static void Main()
        {

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

            // Hyperlight enables binaries to be run by loading an exe into memory , this is useful for debugging, it only works on windows.


            var options = SandboxRunOptions.RunFromGuestBinary;

            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                Console.WriteLine($"Running guest binary {guestBinary} by loading exe into memory");
                using (var exeSandbox = new Sandbox(size, guestBinaryPath, options))
                {
                    exeSandbox.Run(null, 0, 0, 0);
                }
                WaitForUserInput();
            }

            // The binary can also be executed from the host process memory.

            Console.WriteLine($"Running guest binary {guestBinary} from memory");
            options = SandboxRunOptions.RunInProcess;
            using (var inProcSandbox = new Sandbox(size, guestBinaryPath, options))
            {
                inProcSandbox.Run(null, 0, 0, 0);
            }
            WaitForUserInput();

            // if a supported Hypervisor is available, we can run the guest binary in a Hypervisor sandbox.

            if (Sandbox.IsHypervisorPresent())
            {
                Console.WriteLine($"Running guest binary {guestBinary} in Hypervisor sandbox");
                using (var hypervisorSandbox = new Sandbox(size, guestBinaryPath))
                {
                    hypervisorSandbox.Run(null, 0, 0, 0);
                }

                WaitForUserInput();

                // The Sandbox can be reused by specifying the recycle option.

                options = SandboxRunOptions.RecycleAfterRun;

                Console.WriteLine($"Running guest binary {guestBinary} in Hypervisor sandbox 5 times");
                using (var hypervisorSandbox = new Sandbox(size, guestBinaryPath, options))
                {
                    for (var i = 0; i < 5; i++)
                    {
                        hypervisorSandbox.Run(null, 0, 0, 0);
                    }
                }

                WaitForUserInput();

                // The guest binary can invoke methods exported by the host process.
                // The host process can also invoke methods exported by the guest binary.
                // Host methods to be exposed can be static or instance methods on a type or an object.
                // Attributes can be used to control which methods are exposed.
                // Guest methods are represented by delegates on the type.
                // The type or object containing the methods/delegates is passed to the sandbox as a parameter

                guestBinary = "callbackguest.exe";
                guestBinaryPath = Path.Combine(path, guestBinary);

                Console.WriteLine($"Running guest binary {guestBinary} in Hypervisor sandbox with host and guest method execution");
                using (var hypervisorSandbox = new Sandbox(size, guestBinaryPath, typeof(ExposedMethods)))
                {
                    (var returnValue, var _, var _) = hypervisorSandbox.Run(null, 0, 0, 0);
                    Console.WriteLine($"Guest returned {returnValue}");
                }
            }

        }

        static void WaitForUserInput()
        {
            Console.WriteLine("Press any key to continue...");
            if (!Debugger.IsAttached)
            {
                Console.ReadKey();
            }
        }
    }
}
