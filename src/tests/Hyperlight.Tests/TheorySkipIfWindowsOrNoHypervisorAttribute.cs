using System.Runtime.InteropServices;
using Xunit;

namespace Hyperlight.Tests
{
    public sealed class TheorySkipIfWindowsOrNoHypervisorAttribute : TheoryAttribute
    {
        public TheorySkipIfWindowsOrNoHypervisorAttribute()
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                this.Skip = "Running on Windows, and we have temporarily disabled Windows Hyper-V support. See https://github.com/deislabs/hyperlight/issues/845 for more information";
            }
            else if (!Sandbox.IsHypervisorPresent())
            {
                this.Skip = "Linux operating system detected, and no hypervisor is present";
            }
        }
    }
}

