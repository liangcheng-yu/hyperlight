using System.Runtime.InteropServices;
using Xunit;

namespace Hyperlight.Tests
{
    public sealed class FactSkipIfNotLinuxAttribute : FactAttribute
    {
        public FactSkipIfNotLinuxAttribute()
        {
            if (!RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            {
                this.Skip = "Not Runing on Linux.";
            }
        }
    }
}
