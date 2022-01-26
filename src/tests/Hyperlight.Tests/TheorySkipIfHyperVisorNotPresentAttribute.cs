using Xunit;

namespace Hyperlight.Tests
{
    public sealed class TheorySkipIfHyperVisorNotPresentAttribute : TheoryAttribute
    {
        public TheorySkipIfHyperVisorNotPresentAttribute()
        {
            if (!Sandbox.IsHypervisorPresent())
            {
                this.Skip = "Hypervisor is not present on this platform.";
            }
        }
    }
}
