namespace Hyperlight.Native
{
    class HyperlightGuestInterfaceGlue : GuestInterfaceGlue
    {
        readonly Sandbox sandbox;
        public HyperlightGuestInterfaceGlue(Sandbox sandbox)
        {
            this.sandbox = sandbox;
        }

        protected override object DispatchCallFromHost(string functionName, object[] args)
        {
            return sandbox.DispatchCallFromHost(functionName, args);
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

    }
}
