using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using Hyperlight.Core;

namespace Hyperlight
{

    class SimpleStringTable
    {
        IntPtr ptrCurrent;
        readonly IntPtr ptrEnd;
        readonly Dictionary<string, ulong> existingValues = new();
        readonly long offset;

        public SimpleStringTable(IntPtr ptrStart, int length, long offset)
        {
            ptrEnd = IntPtr.Add(ptrStart, length);
            ptrCurrent = ptrStart;
            this.offset = offset;
        }

        public ulong AddString(string s)
        {
            if (existingValues.ContainsKey(s))
            {
                return existingValues[s];
            }

            var data = Encoding.UTF8.GetBytes(s + "\0");
            var ptrNew = IntPtr.Add(ptrCurrent, data.Length);
            if ((long)ptrNew > (long)ptrEnd)
            {
                throw new HyperlightException("Reached end of Buffer");
            }

            Marshal.Copy(data, 0, ptrCurrent, data.Length);
            var adjustedAddress = (ulong)ptrCurrent - (ulong)offset;
            existingValues.Add(s, adjustedAddress);

            ptrCurrent = ptrNew;
            return adjustedAddress;
        }
    }
}
