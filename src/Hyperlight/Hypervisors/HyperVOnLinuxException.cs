using System;

namespace Hyperlight.Hypervisors
{
    public class HyperVOnLinuxException : Exception
    {
        public HyperVOnLinuxException() { }
        public HyperVOnLinuxException(string message) : base(message) { }
        public HyperVOnLinuxException(string message, Exception innerException) : base(message, innerException) { }
    }
}
