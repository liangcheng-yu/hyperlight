using System;

namespace Hyperlight.Native
{
    static class X64
    {
        public const ulong PDE64_PRESENT = (ulong)1;
        public const ulong PDE64_RW = (ulong)1 << 1;
        public const ulong PDE64_USER = (ulong)1 << 2;
        public const ulong PDE64_PS = (ulong)1 << 7;

        public const ulong CR4_PAE = (ulong)1 << 5;
        public const ulong CR4_OSFXSR = (ulong)1 << 9;
        public const ulong CR4_OSXMMEXCPT = (ulong)1 << 10;

        public const ulong CR0_PE = (ulong)1;
        public const ulong CR0_MP = (ulong)1 << 1;
        public const ulong CR0_ET = (ulong)1 << 4;
        public const ulong CR0_NE = (ulong)1 << 5;
        public const ulong CR0_WP = (ulong)1 << 16;
        public const ulong CR0_AM = (ulong)1 << 18;
        public const ulong CR0_PG = (ulong)1 << 31;
        public const ulong EFER_LME = (ulong)1 << 8;
        public const ulong EFER_LMA = (ulong)1 << 10;
    }
}
