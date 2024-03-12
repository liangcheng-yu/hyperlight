using System;
using Xunit;

namespace Hyperlight.Tests
{
    public sealed class FactSkipIfNotCGuest : FactAttribute
    {
        public FactSkipIfNotCGuest()
        {
            if (Environment.GetEnvironmentVariable("guesttype") != "c")
            {
                this.Skip = "Not Running using C guests.";
            }
        }
    }
}
