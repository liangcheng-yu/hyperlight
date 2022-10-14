using System;

namespace HyperlightDependencies
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


    public interface ISandboxRegistration
    {
        public void ExposeHostMethods(Type type);

        public void ExposeAndBindMembers(object instance);

        public void BindGuestFunction(string delegateName, object instance);

        public void ExposeHostMethod(string methodName, object instance);

        public void ExposeHostMethod(string methodName, Type type);

        public T CallGuest<T>(Func<T> func);
    }
}

