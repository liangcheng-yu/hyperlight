using System;
using System.IO;
using System.Runtime.InteropServices;
using Hyperlight;

namespace NativeHost
{
    // The ExposeToGuestAttribute is used to control if members are exposed to the guest in this type or instances of this type when passed to the Sandbox.
    // By default with no attribute, all members are exposed to the guest.
    [ExposeToGuest(false)]
    public class ExposedMethods
    {
        // The Attribute can be used on individual members to give fine grained control.
        // A delegate is used to allow the host to invoke a method in the guest.
        [ExposeToGuest(true)]
        public static Func<string, int>? GuestMethod = null;

        [ExposeToGuest(true)]
        public static int HostMethod(string msg)
        {
            return GuestMethod!($"Host Received: {msg} from Guest");
        }
    }
}
