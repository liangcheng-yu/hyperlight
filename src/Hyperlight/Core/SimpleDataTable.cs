using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using Hyperlight.Core;

namespace Hyperlight
{

    class SimpleDataTable
    {
        IntPtr ptrCurrent;
        readonly IntPtr ptrEnd;
        readonly long offset;

        public SimpleDataTable(IntPtr ptrStart, int length, long offset)
        {
            ptrEnd = IntPtr.Add(ptrStart, length);
            ptrCurrent = ptrStart;
            this.offset = offset;
        }

        public ulong AddString(string s)
        {
            var data = Encoding.UTF8.GetBytes(s + "\0");
            var adjustedAddress = writeData(data);
            return adjustedAddress;
        }

        public ulong AddBytes(byte[] data)
        {
            return writeData(data, true);
        }

        private ulong writeData(byte[] data, bool align = false)
        {
            if (align)
            {
                ptrCurrent = IntPtr.Add(ptrCurrent, 7);
                ptrCurrent = new IntPtr(ptrCurrent.ToInt64() & ~7);
            }

            var ptrNew = IntPtr.Add(ptrCurrent, data.Length);
            if ((long)ptrNew > (long)ptrEnd)
            {
                HyperlightException.LogAndThrowException("Reached end of Buffer", Sandbox.CorrelationId.Value!, GetType().Name);
            }

            Marshal.Copy(data, 0, ptrCurrent, data.Length);
            var adjustedAddress = (ulong)ptrCurrent - (ulong)offset;
            ptrCurrent = ptrNew;
            return adjustedAddress;
        }
    }
}
