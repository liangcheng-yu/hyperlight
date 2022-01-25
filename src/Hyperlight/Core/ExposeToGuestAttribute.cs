using System;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;

namespace Hyperlight
{
    [AttributeUsage(AttributeTargets.Method | AttributeTargets.Class | AttributeTargets.Delegate | AttributeTargets.Field | AttributeTargets.Property, Inherited = true)]
    public sealed class ExposeToGuestAttribute : Attribute
    {
        private readonly bool expose;
        public bool Expose => expose;

        public ExposeToGuestAttribute(bool expose)
        {
            this.expose = expose;
        }
    }

}
