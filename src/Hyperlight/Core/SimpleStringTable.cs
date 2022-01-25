using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;

namespace Hyperlight
{

    class SimpleStringTable
    {
        IntPtr ptrCurrent;
        readonly IntPtr ptrEnd;
        readonly Dictionary<string, ulong> existingValues = new();

        public SimpleStringTable(IntPtr ptrStart, int length)
        {
            ptrEnd = IntPtr.Add(ptrStart, length);
            ptrCurrent = ptrStart;
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
                throw new Exception("Reached end of Buffer");
            }

            Marshal.Copy(data, 0, ptrCurrent, data.Length);
            existingValues.Add(s, (ulong)ptrCurrent);

            var ptrReturn = ptrCurrent;
            ptrCurrent = ptrNew;
            return (ulong)ptrReturn;
        }
    }
}
