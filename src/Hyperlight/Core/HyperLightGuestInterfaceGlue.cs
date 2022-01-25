namespace Hyperlight.Native
{
    class HyperlightGuestInterfaceGlue : GuestInterfaceGlue
    {
        readonly Sandbox sandbox;
        public HyperlightGuestInterfaceGlue(object guestObjectOrType, Sandbox sandbox) : base(guestObjectOrType)
        {
            this.sandbox = sandbox;
        }

        protected override object DispatchCallFromHost(string functionName, object[] args)
        {
            return sandbox.DispatchCallFromHost(functionName, args);
        }
    }
}
