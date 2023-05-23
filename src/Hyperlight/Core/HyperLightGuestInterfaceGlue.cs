using System;

namespace Hyperlight
{
    sealed class HyperlightGuestInterfaceGlue : GuestInterfaceGlue
    {
        readonly Sandbox sandbox;
        public HyperlightGuestInterfaceGlue(Sandbox sandbox)
        {
            this.sandbox = sandbox;
        }

        protected override object DispatchCallFromHost(string functionName, RuntimeTypeHandle returnType, object[] args)
        {
            return sandbox.DispatchCallFromHost(functionName, returnType, args);
        }
        protected override bool EnterDynamicMethod()
        {
            return sandbox.EnterDynamicMethod();
        }
        protected override void ExitDynamicMethod(bool shouldRelease)
        {
            sandbox.ExitDynamicMethod(shouldRelease);
        }
        protected override void ResetState()
        {
            sandbox.ResetState();
        }
        protected override void UpdateCorrelationId()
        {
            sandbox.UpdateCorrelationId();
        }

    }
}
