/// Rust-native API for mutably borrowing C-style arrays as native Rust
/// slices
pub(crate) mod borrowed_slice;
/// Utility for converting ownership back and forth between C-style arrays
/// (represented by a pointer and a length), and a native Rust `Vec`.
pub(crate) mod raw_vec;
