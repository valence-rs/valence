use std::io::Write;
use std::mem::{self, MaybeUninit};
use std::slice;

use anyhow::ensure;

use crate::impls::cautious_capacity;
use crate::{Bounded, Decode, Encode, VarInt};

/// Like tuples, fixed-length arrays are encoded and decoded without a `VarInt`
/// length prefix.
impl<T: Encode, const N: usize> Encode for [T; N] {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        T::encode_slice(self, w)
    }
}

impl<'a, T: Decode<'a>, const N: usize> Decode<'a> for [T; N] {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        // TODO: rewrite using std::array::try_from_fn when stabilized?

        let mut data: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };

        for (i, elem) in data.iter_mut().enumerate() {
            match T::decode(r) {
                Ok(val) => {
                    elem.write(val);
                }
                Err(e) => {
                    // Call destructors for values decoded so far.
                    for elem in &mut data[..i] {
                        unsafe { elem.assume_init_drop() };
                    }
                    return Err(e);
                }
            }
        }

        // All values in `data` are initialized.
        unsafe { Ok(mem::transmute_copy(&data)) }
    }
}

/// References to fixed-length arrays are not length prefixed.
impl<'a, const N: usize> Decode<'a> for &'a [u8; N] {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        ensure!(
            r.len() >= N,
            "not enough data to decode u8 array of length {N}"
        );

        let (res, remaining) = r.split_at(N);
        let arr = <&[u8; N]>::try_from(res).unwrap();
        *r = remaining;
        Ok(arr)
    }
}

impl<T: Encode> Encode for [T] {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let len = self.len();
        ensure!(
            i32::try_from(len).is_ok(),
            "length of {} slice exceeds i32::MAX (got {len})",
            std::any::type_name::<T>()
        );

        VarInt(len as i32).encode(&mut w)?;

        T::encode_slice(self, w)
    }
}

impl<T: Encode, const MAX_LEN: usize> Encode for Bounded<&'_ [T], MAX_LEN> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let len = self.len();
        ensure!(
            len <= MAX_LEN,
            "length of {} slice exceeds max of {MAX_LEN} (got {len})",
            std::any::type_name::<T>(),
        );

        VarInt(len as i32).encode(&mut w)?;

        T::encode_slice(self, w)
    }
}

impl<'a> Decode<'a> for &'a [u8] {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(len >= 0, "attempt to decode slice with negative length");
        let len = len as usize;
        ensure!(
            len <= r.len(),
            "not enough data remaining to decode byte slice (slice len is {len}, but input len is \
             {})",
            r.len()
        );

        let (res, remaining) = r.split_at(len);
        *r = remaining;
        Ok(res)
    }
}

impl<'a, const MAX_LEN: usize> Decode<'a> for Bounded<&'a [u8], MAX_LEN> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let res = <&[u8]>::decode(r)?;

        ensure!(
            res.len() <= MAX_LEN,
            "length of decoded byte slice exceeds max of {MAX_LEN} (got {})",
            res.len()
        );

        Ok(Bounded(res))
    }
}

impl<'a> Decode<'a> for &'a [i8] {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let bytes = <&[u8]>::decode(r)?;

        // SAFETY: i8 and u8 have the same layout.
        let bytes = unsafe { slice::from_raw_parts(bytes.as_ptr() as *const i8, bytes.len()) };

        Ok(bytes)
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.as_slice().encode(w)
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Vec<T> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(len >= 0, "attempt to decode Vec with negative length");
        let len = len as usize;

        let mut vec = Vec::with_capacity(cautious_capacity::<T>(len));

        for _ in 0..len {
            vec.push(T::decode(r)?);
        }

        Ok(vec)
    }
}

impl<'a, T: Decode<'a>, const MAX_LEN: usize> Decode<'a> for Bounded<Vec<T>, MAX_LEN> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(len >= 0, "attempt to decode Vec with negative length");
        let len = len as usize;

        ensure!(
            len <= MAX_LEN,
            "length of Vec exceeds max of {MAX_LEN} (got {len})"
        );

        let mut vec = Vec::with_capacity(len);

        for _ in 0..len {
            vec.push(T::decode(r)?);
        }

        Ok(Bounded(vec))
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Box<[T]> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(Vec::decode(r)?.into_boxed_slice())
    }
}

impl<'a, T: Decode<'a>, const MAX_LEN: usize> Decode<'a> for Bounded<Box<[T]>, MAX_LEN> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(Bounded::<Vec<_>, MAX_LEN>::decode(r)?.map_into())
    }
}
