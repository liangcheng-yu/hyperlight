using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Hyperlight
{
    class HyperlightPEB
    {
        struct Header
        {
            public ulong CountFunctions;
            public ulong DispatchFunction;
        }
        struct FunctionDefinition
        {
            public ulong FunctionName;
            public ulong FunctionSignature;
            public ulong Flags;
        }

        class FunctionDetails
        {
            public string FunctionName { get; set; }
            public string FunctionSignature { get; set; }
            public ulong Flags { get; set; }

        }
        readonly List<FunctionDetails> listFunctions = new();

        public void AddFunction(string functionName, string functionSignature, ulong flags)
        {
            listFunctions.Add(new FunctionDetails() { FunctionName = functionName, FunctionSignature = functionSignature, Flags = flags });
        }
        public void WriteToMemory(IntPtr ptr, int length, ulong offset)
        {
            var header = new Header() { CountFunctions = (ulong)listFunctions.Count, DispatchFunction = 0 };
            var headerSize = Marshal.SizeOf<Header>();
            var functionDefinitionSize = Marshal.SizeOf<FunctionDefinition>();
            var totalHeaderSize = headerSize + (int)header.CountFunctions * functionDefinitionSize;
            if (totalHeaderSize > length)
            {
                throw new Exception("Not enough memory for header structures");
            }

            var stringTable = new SimpleStringTable(IntPtr.Add(ptr, totalHeaderSize), length - totalHeaderSize, offset);

            Marshal.StructureToPtr(header, ptr, false);
            ptr += headerSize;

            foreach (var func in listFunctions)
            {
                var fd = new FunctionDefinition() { FunctionName = stringTable.AddString(func.FunctionName), FunctionSignature = stringTable.AddString(func.FunctionSignature), Flags = func.Flags };
                Marshal.StructureToPtr(fd, ptr, false);
                ptr += functionDefinitionSize;
            }
        }
    }


}
