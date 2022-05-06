using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Hyperlight
{
    public interface ISandboxRegistration
    {
        public void ExposeHostMethods(Type type);

        public void ExposeAndBindMembers(object instance);

        public void BindGuestFunction(string delegateName, object instance);

        public void ExposeHostMethod(string methodName, object instance);

        public void ExposeHostMethod(string methodName, Type type);
    }
}
