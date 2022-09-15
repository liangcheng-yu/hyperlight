using System;
using HyperlightDependencies;

namespace Hyperlight.Core
{
    [ExposeToGuest(true)]
    public sealed class HyperLightExports
    {
#pragma warning disable CA1024 // Use properties where appropriate - Intentional as properties cannot be exposed to guest
        public static long GetTickCount()
#pragma warning restore CA1024 // Use properties where appropriate
        {
            return Environment.TickCount64;
        }
    }
}
