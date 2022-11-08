// TODO: implement BoundedFloat when floats are permitted in const generics.

use std::io::Write;

use anyhow::ensure;

use crate::{Decode, Encode, Result};

/// An integer with a minimum and maximum value known at compile time. `T` is
/// the underlying integer type.
///
/// If the value is not in bounds, an error is generated while
/// encoding or decoding.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct BoundedInt<T, const MIN: i128, const MAX: i128>(pub T);

impl<T, const MIN: i128, const MAX: i128> Encode for BoundedInt<T, MIN, MAX>
where
    T: Encode + Clone + Into<i128>,
{
    fn encode(&self, w: impl Write) -> Result<()> {
        let n = self.0.clone().into();

        ensure!(
            (MIN..=MAX).contains(&n),
            "integer is not in bounds while encoding (got {n}, expected {MIN}..={MAX})"
        );

        self.0.encode(w)
    }

    fn encoded_len(&self) -> usize {
        self.0.encoded_len()
    }
}

impl<'a, T, const MIN: i128, const MAX: i128> Decode<'a> for BoundedInt<T, MIN, MAX>
where
    T: Decode<'a> + Clone + Into<i128>,
{
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        let res = T::decode(r)?;
        let n = res.clone().into();

        ensure!(
            (MIN..=MAX).contains(&n),
            "integer is not in bounds while decoding (got {n}, expected {MIN}..={MAX})"
        );

        Ok(Self(res))
    }
}

/// A string with a minimum and maximum character length known at compile time.
/// `S` is the underlying string type which is anything that implements
/// `AsRef<str>`.
///
/// If the string is not in bounds, an error is generated while
/// encoding or decoding.
///
/// Note that the length is a count of the _characters_ in the string, not
/// bytes.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct BoundedString<S, const MIN: usize, const MAX: usize>(pub S);

impl<S, const MIN: usize, const MAX: usize> Encode for BoundedString<S, MIN, MAX>
where
    S: AsRef<str>,
{
    fn encode(&self, w: impl Write) -> Result<()> {
        let s = self.0.as_ref();
        let cnt = s.chars().count();

        ensure!(
            (MIN..=MAX).contains(&s.chars().count()),
            "char count of string is out of bounds while encoding (got {cnt}, expected \
             {MIN}..={MAX})"
        );

        s.encode(w)?;

        Ok(())
    }

    fn encoded_len(&self) -> usize {
        self.0.as_ref().encoded_len()
    }
}

impl<'a, S, const MIN: usize, const MAX: usize> Decode<'a> for BoundedString<S, MIN, MAX>
where
    S: Decode<'a> + AsRef<str>,
{
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        let s = S::decode(r)?;
        let cnt = s.as_ref().chars().count();

        ensure!(
            (MIN..=MAX).contains(&cnt),
            "char count of string is out of bounds while decoding (got {cnt}, expected \
             {MIN}..={MAX})"
        );

        Ok(Self(s))
    }
}
