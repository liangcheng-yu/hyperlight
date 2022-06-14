using Xunit;

namespace Hyperlight.Tests
{
    public sealed class FactSkipIfHypervisorNotPresentAttribute : FactAttribute
    {
        public FactSkipIfHypervisorNotPresentAttribute()
        {
            if (!Sandbox.IsHypervisorPresent())
            {
                this.Skip = "Hypervisor is not present on this platform.";
            }
        }
    }
}
