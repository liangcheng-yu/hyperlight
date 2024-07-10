pub(crate) const FP_CONTROL_WORD_DEFAULT: u16 = 0x37f; // mask all fp-exception, set rounding to nearest, set precision to 64-bit
pub(crate) const FP_TAG_WORD_DEFAULT: u8 = 0xff; // each 8 of x87 fpu registers is empty
pub(crate) const MXCSR_DEFAULT: u32 = 0x1f80; // mask simd fp-exceptions, clear exception flags, set rounding to nearest, disable flush-to-zero mode, disable denormals-are-zero mode
