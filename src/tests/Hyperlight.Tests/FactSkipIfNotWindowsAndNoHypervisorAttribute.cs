using System.Runtime.InteropServices;
using Xunit;

namespace Hyperlight.Tests
{
    public sealed class FactSkipIfNotWindowsAndNoHypervisorAttribute : FactAttribute
    {
        public FactSkipIfNotWindowsAndNoHypervisorAttribute()
        {
            if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows) && !Sandbox.IsHypervisorPresent())
            {
                this.Skip = "Not runing on Windows and hypervisor is not present on this platform.";
            }
        }
    }
}

