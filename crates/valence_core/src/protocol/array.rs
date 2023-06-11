use std::io::Write;

use anyhow::ensure;

use crate::protocol::var_int::VarInt;
use crate::protocol::{Decode, Encode};

/// A fixed-size array encoded and decoded with a [`VarInt`] length prefix.
///
/// This is used when the length of the array is known statically, but a
/// length prefix is needed anyway.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(transparent)]
pub struct LengthPrefixedArray<T, const N: usize>(pub [T; N]);

impl<T: Encode, const N: usize> Encode for LengthPrefixedArray<T, N> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        VarInt(N as i32).encode(&mut w)?;
        self.0.encode(w)
    }
}

impl<'a, T: Decode<'a>, const N: usize> Decode<'a> for LengthPrefixedArray<T, N> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(len == N as i32, "unexpected array length of {len}");

        <[T; N]>::decode(r).map(LengthPrefixedArray)
    }
}

impl<T, const N: usize> From<[T; N]> for LengthPrefixedArray<T, N> {
    fn from(value: [T; N]) -> Self {
        Self(value)
    }
}

impl<T, const N: usize> From<LengthPrefixedArray<T, N>> for [T; N] {
    fn from(value: LengthPrefixedArray<T, N>) -> Self {
        value.0
    }
}
