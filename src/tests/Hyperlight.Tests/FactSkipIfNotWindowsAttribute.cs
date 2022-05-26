using System.Runtime.InteropServices;
using Xunit;

namespace Hyperlight.Tests
{
    public sealed class FactSkipIfNotWindowsAttribute : FactAttribute
    {
        public FactSkipIfNotWindowsAttribute()
        {
            if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                this.Skip = "Not Runing on Windows.";
            }
        }
    }
}
