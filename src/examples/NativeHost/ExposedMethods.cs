using System;
using Hyperlight;
using HyperlightDependencies;

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
        public Func<string, int>? GuestMethod = null;

        [ExposeToGuest(true)]
        public Func<String, int>? PrintOutput = null;

        [ExposeToGuest(true)]
        public int HostMethod(string msg)
        {
            // This method is being changed because of https://github.com/deislabs/hyperlight/issues/909
            // whilst calling back still works from C API for mshv and WHP after https://github.com/deislabs/hyperlight/pull/1528 
            // it is broken for KVM so rather than amend this for KVM only we will disable it for all for now
            Console.WriteLine($"Host Received: {msg} from Guest");
            return string.IsNullOrEmpty(msg) ? 0 : msg.Length;
            //return PrintOutput!($"Host Received: {msg} from Guest");
        }
    }
}
