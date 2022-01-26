using System.Runtime.InteropServices;
using Xunit;

namespace Hyperlight.Tests
{
    public sealed class TheorySkipIfNotWindowsAttribute : TheoryAttribute
    {
        public TheorySkipIfNotWindowsAttribute()
        {
            if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                this.Skip = "Not Runing on Windows.";
            }
        }
    }
}
