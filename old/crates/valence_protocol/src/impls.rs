//! Implementations of [`Encode`](crate::Encode) and [`Decode`](crate::Decode)
//! on foreign types.

mod map;
mod math;
mod other;
mod pointer;
mod primitive;
mod sequence;
mod string;
mod tuple;

use std::mem;

/// Prevents preallocating too much memory in case we get a malicious or invalid
/// sequence length.
fn cautious_capacity<Element>(size_hint: usize) -> usize {
    const MAX_PREALLOC_BYTES: usize = 1024 * 1024;

    if mem::size_of::<Element>() == 0 {
        0
    } else {
        size_hint.min(MAX_PREALLOC_BYTES / mem::size_of::<Element>())
    }
}
