using System;
namespace Hyperlight
{
    [AttributeUsage(AttributeTargets.Method | AttributeTargets.Class | AttributeTargets.Delegate | AttributeTargets.Field | AttributeTargets.Property, Inherited = true)]
#pragma warning disable CA1813 // Avoid unserializable object exceptions
    public class ExposeToGuestAttribute : Attribute
#pragma warning restore CA1813 // Avoid unserializable object exceptions
    {
        private readonly bool expose;
        public bool Expose => expose;

        public ExposeToGuestAttribute(bool expose)
        {
            this.expose = expose;
        }
    }
}
