using System;
using System.Runtime.InteropServices;

namespace Hyperlight.Wrapper
{
    [StructLayout(LayoutKind.Sequential, Pack = 8, CharSet = CharSet.Ansi)]
    public struct PEHeaders : IEquatable<PEHeaders>
    {
        /// Stack reserve size.
        public readonly UInt64 StackReserve;

        /// Stack commit size.
        public readonly UInt64 StackCommit;

        /// Heap reserve size.
        public readonly UInt64 HeapReserve;

        /// Heap commit size.
        public readonly UInt64 HeapCommit;

        /// EntryPoint offset.
        public readonly UInt64 EntryPointOffset;

        /// Preferred load address.
        public readonly UInt64 PreferredLoadAddress;

        public static bool operator ==(
            PEHeaders lhs,
            PEHeaders rhs
        )
        {
            return lhs.Equals(rhs);
        }

        public static bool operator !=(
            PEHeaders lhs,
            PEHeaders rhs
        )
        {
            return !lhs.Equals(rhs);
        }

        public bool Equals(PEHeaders other)
        {
            return (
                other.EntryPointOffset == this.EntryPointOffset &&
                other.HeapCommit == this.HeapCommit &&
                other.HeapReserve == this.HeapReserve &&
                other.PreferredLoadAddress == this.PreferredLoadAddress &&
                other.StackCommit == this.StackCommit &&
                other.StackReserve == this.StackReserve
            );
        }

        public override bool Equals(Object? obj)
        {
            //Check for null and compare run-time types.
            if ((obj == null) || !this.GetType().Equals(obj.GetType()))
            {
                return false;
            }
            var other = (PEHeaders)obj;
            return this.Equals(other);
        }

        public override int GetHashCode()
        {
            return (int)(
                this.EntryPointOffset +
                this.HeapCommit +
                this.HeapReserve +
                this.StackCommit +
                this.StackReserve +
                this.PreferredLoadAddress
            );
        }
    }

    public sealed class PEInfo : IDisposable
    {
        private readonly Context ctxWrapper;
        private readonly ByteArray payload;
        public int PayloadLength { get; private set; }
        public Handle handleWrapper { get; private set; }
        private bool disposed;

        public PEInfo(Context ctx, String filename)
        {
            this.ctxWrapper = ctx;

            var contents = System.IO.File.ReadAllBytes(filename);
            this.payload = new ByteArray(ctx, contents);
            this.PayloadLength = contents.Length;
            this.handleWrapper = new Handle(
                this.ctxWrapper,
                pe_parse(
                    this.ctxWrapper.ctx,
                    this.payload.handleWrapper.handle
                )
            );
            this.handleWrapper.ThrowIfError();
        }

        public void Dispose()
        {
            this.Dispose(disposing: true);
            GC.SuppressFinalize(this);
        }

        private void Dispose(bool disposing)
        {
            if (!this.disposed)
            {
                if (disposing)
                {
                    this.handleWrapper.Dispose();
                    this.payload.Dispose();
                }
                this.disposed = true;
            }
        }

        /// Retrieves the PE file headers.
        public PEHeaders GetHeaders()
        {
            return pe_get_headers(this.ctxWrapper.ctx, this.handleWrapper.handle);
        }

        /// Retrieves the original payload of the PE file, without relocations.
        public byte[] GetPayload()
        {
            return this.payload.GetContents();
        }

        /// Updates the PE file payload and relocates it to the specified load address
        /// returning the relocated payload.
        public byte[] GetPayload(ulong addressToLoadAt)
        {
            var arrHdl = new Handle(
                 this.ctxWrapper,
                 pe_relocate(this.ctxWrapper.ctx, this.handleWrapper.handle, this.payload.handleWrapper.handle, addressToLoadAt)
             );
            try
            {
                arrHdl.ThrowIfError();
                return this.payload.GetContents();
            }
            finally
            {
                arrHdl.Dispose();
            }
        }

#pragma warning disable CA5393 // Use of unsafe DllImportSearchPath value AssemblyDirectory
        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle pe_parse(
            NativeContext ctx,
            NativeHandle byte_array_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern PEHeaders pe_get_headers(
            NativeContext ctx,
            NativeHandle pe_handle
        );

        [DllImport("hyperlight_host", SetLastError = false, ExactSpelling = true)]
        [DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory)]
        private static extern NativeHandle pe_relocate(
            NativeContext ctx,
            NativeHandle pe_handle,
            NativeHandle bye_array_handle,
            ulong addr_to_load_at
        );
#pragma warning restore CA5393
    }
}
