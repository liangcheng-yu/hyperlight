using System;
using System.IO;
using System.Runtime.InteropServices;
using Hyperlight.Native;

namespace Hyperlight
{
    public enum ImageRel
    {
        IMAGE_REL_BASED_ABSOLUTE = 0,
        IMAGE_REL_BASED_DIR64 = 10,
    }

    [Flags]
    public enum ImageFileCharacteristics
    {
        IMAGE_FILE_RELOCS_STRIPPED = 0x0001,
        IMAGE_FILE_EXECUTABLE_IMAGE = 0x0002,
        IMAGE_FILE_LINE_NUMS_STRIPPED = 0x0004,
        IMAGE_FILE_LOCAL_SYMS_STRIPPED = 0x0008,
        IMAGE_FILE_AGGRESSIVE_WS_TRIM = 0x0010,
        IMAGE_FILE_LARGE_ADDRESS_AWARE = 0x0020,
        IMAGE_FILE_RESERVED = 0x0040,
        IMAGE_FILE_BYTES_REVERSED_LO = 0x0080,
        IMAGE_FILE_32BIT_MACHINE = 0x0100,
        IMAGE_FILE_DEBUG_STRIPPED = 0x0200,
        IMAGE_FILE_REMOVABLE_RUN_FROM_SWAP = 0x0400,
        IMAGE_FILE_NET_RUN_FROM_SWAP = 0x0800,
        IMAGE_FILE_SYSTEM = 0x1000,
        IMAGE_FILE_DLL = 0x2000,
        IMAGE_FILE_UP_SYSTEM_ONLY = 0x4000,
        IMAGE_FILE_BYTES_REVERSED_HI = 0x8000,
    }

    [StructLayout(LayoutKind.Sequential, Pack = 4)]
    internal struct ImageDataDirectory
    {
        public uint VirtualAddress;
        public uint Size;
    }

    [StructLayout(LayoutKind.Sequential, Pack = 4)]
    internal struct ImageBaseRelocation
    {
        public uint VirtualAddress;
        public uint SizeOfBlock;
    }

    internal class PEInfo
    {
        const ushort IMAGE_FILE_MACHINE_AMD64 = 0x8664;
        const int ELFANEW_OFFSET = 0x3C;
        const int IMAGE_MACHINE_TYPE_OFFSET = 0x04;
        const int IMAGE_CHARACTERISTCS_OFFSET = 0x16;
        const int IMAGE_MAGICNUMBER_OFFSET = 0x18;
        const int IMAGE_ENTRYPOINT_OFFSET = 0x28;
        const int IMAGE_LOADADDRESS_OFFSET = 0x30;
        const int IMAGE_RELOCATIONHEADER_OFFSET = 0xB0;
        const int IMAGE_PE32PLUS = 0x20B;
        const int IMAGE_STACK_RESERVE = 0x60;
        const int IMAGE_STACK_COMMIT = 0x68;
        const int IMAGE_HEAP_RESERVE = 0x70;
        const int IMAGE_HEAP_COMMIT = 0x78;

        public ulong PrefferedLoadAddress;
        public uint EntryPointOffset;
        public readonly byte[] Payload;
        public readonly byte[] HyperVisorPayload;
        public int RelocationHeaderOffset;

        public long HeapReserve { get; init; }
        public long HeapCommit { get; init; }
        public long StackReserve { get; init; }
        public long StackCommit { get; init; }


        internal PEInfo(string guestBinaryPath, ulong hyperVisorCodeAddress)
        {

            // see http://download.microsoft.com/download/9/c/5/9c5b2167-8017-4bae-9fde-d599bac8184a/pecoff_v83.docx for details of PE format.

            Payload = File.ReadAllBytes(guestBinaryPath);
            if (Payload[0] == (byte)'M' && Payload[1] == (byte)'Z')
            {
                Payload[0] = (byte)'J'; // Mark first byte as '0' so we know we are running in hyperlight VM and not as real windows exe
            }
            else
            {
                throw new ArgumentException($"File {guestBinaryPath} is not a PE File");
            }

            var e_lfanew = BitConverter.ToInt32(Payload, ELFANEW_OFFSET);
            var machineType = BitConverter.ToUInt16(Payload, e_lfanew + IMAGE_MACHINE_TYPE_OFFSET);
            // Validate that this is an x64 binary.
            if (machineType != IMAGE_FILE_MACHINE_AMD64)
            {
                throw new ArgumentException($"File {guestBinaryPath} is not a x64 File");
            }

            var characteristics = BitConverter.ToUInt16(Payload, e_lfanew + IMAGE_CHARACTERISTCS_OFFSET);
            var imageFileCharacteristics = (ImageFileCharacteristics)characteristics;
            // validate that this is an exe and is not forced to load at its preferred base address.
            if (!imageFileCharacteristics.HasFlag(ImageFileCharacteristics.IMAGE_FILE_EXECUTABLE_IMAGE))
            {
                throw new ArgumentException($"File {guestBinaryPath} is not an executable image");
            }

            if (imageFileCharacteristics.HasFlag(ImageFileCharacteristics.IMAGE_FILE_RELOCS_STRIPPED))
            {
                throw new ArgumentException($"File {guestBinaryPath} should not be force to load at fixed address");
            }

            // validate that the magic number is 0x20b - this should be a PE32+ file.
            var magicNumber = BitConverter.ToInt16(Payload, e_lfanew + IMAGE_MAGICNUMBER_OFFSET);
            if (magicNumber != IMAGE_PE32PLUS)
            {
                throw new ArgumentException($"File {guestBinaryPath} is not a PE32+ formatted file");
            }

            StackReserve = BitConverter.ToInt64(Payload, e_lfanew + IMAGE_STACK_RESERVE);
            StackCommit = BitConverter.ToInt64(Payload, e_lfanew + IMAGE_STACK_COMMIT);
            HeapReserve = BitConverter.ToInt64(Payload, e_lfanew + IMAGE_HEAP_RESERVE);
            HeapCommit = BitConverter.ToInt64(Payload, e_lfanew + IMAGE_HEAP_COMMIT);

            EntryPointOffset = BitConverter.ToUInt32(Payload, e_lfanew + IMAGE_ENTRYPOINT_OFFSET);
            PrefferedLoadAddress = BitConverter.ToUInt64(Payload, e_lfanew + IMAGE_LOADADDRESS_OFFSET);
            RelocationHeaderOffset = e_lfanew + IMAGE_RELOCATIONHEADER_OFFSET;

            // Fix up the address for running in HyperVisor
            // this is done here as it only needs doing once.

            HyperVisorPayload = new byte[Payload.Length];
            Payload.AsSpan().CopyTo(HyperVisorPayload);
            PatchExeRelocations(HyperVisorPayload.AsSpan(), hyperVisorCodeAddress);
        }

        internal unsafe void PatchExeRelocations(Span<byte> payload, ulong hyperVisorCodeAddress)
        {
            fixed (byte* pPayload = &MemoryMarshal.GetReference(payload))
            {
                PatchExeRelocations(hyperVisorCodeAddress, (ulong)pPayload);
            }
        }

        // When performing patching of relocations for an in process loaded exe the addresstoloadat and the currentaddress are the same.

        internal void PatchExeRelocations(ulong loadedAddress)
        {
            PatchExeRelocations(loadedAddress, loadedAddress);
        }

        /// <summary>
        /// This method updates an exe to fix all the relocatable references to reflect the address where the exe is loaded.
        /// </summary>
        /// <param name="addressToLoadAt">This is the address where the exe is loaded at or is going to be loaded at</param>
        /// <param name="currentAddress">This is the current address of the exe.</param>

        void PatchExeRelocations(ulong addressToLoadAt, ulong currentAddress)
        {

            // see https://stackoverflow.com/questions/17436668/how-are-pe-base-relocations-build-up

            // If the exe is loading/loaded at its preferred address there is nothing to do
            if (PrefferedLoadAddress == addressToLoadAt)
            {
                return;
            }

            var pImageDataDirectory = (IntPtr)(currentAddress + (ulong)RelocationHeaderOffset);
            var imageDataDirectory = Marshal.PtrToStructure<ImageDataDirectory>(pImageDataDirectory);

            // If there is no reloc data nothing to do (dont think this should be possible as we check that the exe is relocatable) 

            if (imageDataDirectory.Size == 0)
            {
                return;
            }

            // Get the address of the first relocation block

            var pImageBaseRelocation = (IntPtr)(currentAddress + (ulong)imageDataDirectory.VirtualAddress);

            ulong difference = addressToLoadAt - PrefferedLoadAddress;

            uint sizeProcessed = 0;

            while (sizeProcessed < imageDataDirectory.Size)
            {
                // get the relocation block  

                var imageBaseRelocation = Marshal.PtrToStructure<ImageBaseRelocation>(pImageBaseRelocation);

                // if there are no entries we are done (dont think this can happen as we check if we have processed everything in the while condition)

                if (imageBaseRelocation.SizeOfBlock == 0)
                {
                    break;
                }

                // calculate the base address
                var baseAddress = (IntPtr)(currentAddress + imageBaseRelocation.VirtualAddress);

                // the number of relocations is the size of block minus the 8 byte header divided by 2 (each relocation is 2 bytes)
                var numberOfRelocations = (imageBaseRelocation.SizeOfBlock - 8) / 2;

                for (var i = 0; i < numberOfRelocations; i++)
                {
                    // Get the Relocation information
                    var pRelocationInfo = pImageBaseRelocation + 8 + (2 * i);
                    var relocationInfo = Marshal.PtrToStructure<ushort>(pRelocationInfo);
                    // the lowest 12 bits is the relocatio offset for this entry
                    var relocOffset = relocationInfo & 0x0fff;
                    // the highest 4 bits is the type
                    var relocType = (relocationInfo & 0xf000) >> 12;

                    // Check that this is IMAGE_REL_BASED_DIR64 relocatin type
                    if ((ImageRel)relocType == ImageRel.IMAGE_REL_BASED_DIR64)
                    {
                        // get the address to read the existing address from and write the new address to
                        var targetAddr = (IntPtr)(baseAddress + relocOffset);
                        // the new address is the existing address +/- the difference between the preferred address and the loaded address
                        var newAddr = (IntPtr)((ulong)Marshal.ReadIntPtr(targetAddr) + difference);
                        // Write the new address
                        Marshal.WriteIntPtr(targetAddr, newAddr);
                    }
                    else
                    {
                        // IMAGE_REL_BASED_ABSOLUTE is padding at end of block, the last block is empty.
                        if ((ImageRel)relocType != ImageRel.IMAGE_REL_BASED_ABSOLUTE)
                        {
                            throw new Exception($"Unexpected reloc type: {relocType}");
                        }
                    }
                }

                sizeProcessed += imageBaseRelocation.SizeOfBlock;

                // Read the next block.

                pImageBaseRelocation = (IntPtr)((ulong)pImageBaseRelocation + imageBaseRelocation.SizeOfBlock);

            }
        }
    }
}
