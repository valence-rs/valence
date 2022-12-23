use std::borrow::Cow;
use std::io::Write;
use std::mem::MaybeUninit;
use std::rc::Rc;
use std::sync::Arc;
use std::{io, mem};

use anyhow::ensure;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use uuid::Uuid;
use valence_nbt::Compound;

use crate::{Decode, Encode, Result, VarInt, MAX_PACKET_SIZE};

// ==== Primitive ==== //

impl Encode for bool {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_u8(*self as u8)?)
    }

    fn write_slice(slice: &[bool], mut w: impl Write) -> io::Result<()> {
        let bytes: &[u8] = unsafe { mem::transmute(slice) };
        w.write_all(bytes)
    }

    const HAS_WRITE_SLICE: bool = true;
}

impl Decode<'_> for bool {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        let n = r.read_u8()?;
        ensure!(n <= 1, "boolean is not 0 or 1");
        Ok(n == 1)
    }
}

impl Encode for u8 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_u8(*self)?)
    }

    fn write_slice(slice: &[u8], mut w: impl Write) -> io::Result<()> {
        w.write_all(slice)
    }

    const HAS_WRITE_SLICE: bool = true;
}

impl Decode<'_> for u8 {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(r.read_u8()?)
    }
}

impl Encode for i8 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_i8(*self)?)
    }

    fn write_slice(slice: &[i8], mut w: impl Write) -> io::Result<()>
    where
        Self: Sized,
    {
        let bytes: &[u8] = unsafe { mem::transmute(slice) };
        w.write_all(bytes)
    }

    const HAS_WRITE_SLICE: bool = true;
}

impl Decode<'_> for i8 {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(r.read_i8()?)
    }
}

impl Encode for u16 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_u16::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for u16 {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(r.read_u16::<BigEndian>()?)
    }
}

impl Encode for i16 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_i16::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for i16 {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(r.read_i16::<BigEndian>()?)
    }
}

impl Encode for u32 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_u32::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for u32 {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(r.read_u32::<BigEndian>()?)
    }
}

impl Encode for i32 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_i32::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for i32 {
    fn decode(r: &mut &'_ [u8]) -> Result<Self> {
        Ok(r.read_i32::<BigEndian>()?)
    }
}

impl Encode for u64 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_u64::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for u64 {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(r.read_u64::<BigEndian>()?)
    }
}

impl Decode<'_> for i64 {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(r.read_i64::<BigEndian>()?)
    }
}

impl Encode for u128 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_u128::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for u128 {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(r.read_u128::<BigEndian>()?)
    }
}

impl Encode for i128 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_i128::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for i128 {
    fn decode(r: &mut &'_ [u8]) -> Result<Self> {
        Ok(r.read_i128::<BigEndian>()?)
    }
}

impl Encode for i64 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_i64::<BigEndian>(*self)?)
    }
}

impl Encode for f32 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        ensure!(
            self.is_finite(),
            "attempt to encode non-finite f32 ({})",
            self
        );
        Ok(w.write_f32::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for f32 {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        let f = r.read_f32::<BigEndian>()?;
        ensure!(f.is_finite(), "attempt to decode non-finite f32 ({f})");
        Ok(f)
    }
}

impl Encode for f64 {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        ensure!(
            self.is_finite(),
            "attempt to encode non-finite f64 ({})",
            self
        );
        Ok(w.write_f64::<BigEndian>(*self)?)
    }
}

impl Decode<'_> for f64 {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        let f = r.read_f64::<BigEndian>()?;
        ensure!(f.is_finite(), "attempt to decode non-finite f64 ({f})");
        Ok(f)
    }
}

// ==== Pointer ==== //

impl<T: Encode + ?Sized> Encode for &T {
    fn encode(&self, w: impl Write) -> Result<()> {
        (**self).encode(w)
    }
}

impl<T: Encode + ?Sized> Encode for &mut T {
    fn encode(&self, w: impl Write) -> Result<()> {
        (**self).encode(w)
    }
}

impl<T: Encode + ?Sized> Encode for Box<T> {
    fn encode(&self, w: impl Write) -> Result<()> {
        self.as_ref().encode(w)
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Box<T> {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        T::decode(r).map(Box::new)
    }
}

impl<T: Encode + ?Sized> Encode for Rc<T> {
    fn encode(&self, w: impl Write) -> Result<()> {
        self.as_ref().encode(w)
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Rc<T> {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        T::decode(r).map(Rc::new)
    }
}

impl<T: Encode + ?Sized> Encode for Arc<T> {
    fn encode(&self, w: impl Write) -> Result<()> {
        self.as_ref().encode(w)
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Arc<T> {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        T::decode(r).map(Arc::new)
    }
}

// ==== Tuple ==== //

macro_rules! impl_tuple {
    ($($ty:ident)*) => {
        #[allow(non_snake_case)]
        impl<$($ty: Encode,)*> Encode for ($($ty,)*) {
            fn encode(&self, mut _w: impl Write) -> Result<()> {
                let ($($ty,)*) = self;
                $(
                    $ty.encode(&mut _w)?;
                )*
                Ok(())
            }
        }

        impl<'a, $($ty: Decode<'a>,)*> Decode<'a> for ($($ty,)*) {
            fn decode(_r: &mut &'a [u8]) -> Result<Self> {
                Ok(($($ty::decode(_r)?,)*))
            }
        }
    }
}

impl_tuple!();
impl_tuple!(A);
impl_tuple!(A B);
impl_tuple!(A B C);
impl_tuple!(A B C D);
impl_tuple!(A B C D E);
impl_tuple!(A B C D E F);
impl_tuple!(A B C D E F G);
impl_tuple!(A B C D E F G H);
impl_tuple!(A B C D E F G H I);
impl_tuple!(A B C D E F G H I J);
impl_tuple!(A B C D E F G H I J K);
impl_tuple!(A B C D E F G H I J K L);

// ==== Sequence ==== //

/// Like tuples, arrays are encoded and decoded without a VarInt length prefix.
impl<const N: usize, T: Encode> Encode for [T; N] {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        if T::HAS_WRITE_SLICE {
            return Ok(T::write_slice(self, w)?);
        }

        for t in self {
            t.encode(&mut w)?;
        }

        Ok(())
    }
}

impl<'a, const N: usize, T: Decode<'a>> Decode<'a> for [T; N] {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        // TODO: rewrite using std::array::try_from_fn when stabilized?
        // TODO: specialization for [f64; 3] improved performance.

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
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
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
    fn encode(&self, mut w: impl Write) -> Result<()> {
        let len = self.len();
        ensure!(
            len <= i32::MAX as usize,
            "length of slice ({len}) exceeds i32::MAX"
        );

        VarInt(len as i32).encode(&mut w)?;

        if T::HAS_WRITE_SLICE {
            return Ok(T::write_slice(self, w)?);
        }

        for t in self {
            t.encode(&mut w)?;
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for &'a [u8] {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(len >= 0, "attempt to decode slice with negative length");
        let len = len as usize;
        ensure!(r.len() >= len, "not enough data remaining to decode slice");

        let (res, remaining) = r.split_at(len);
        *r = remaining;
        Ok(res)
    }
}

impl<'a> Decode<'a> for &'a [i8] {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        let unsigned_bytes = <&[u8]>::decode(r)?;
        let signed_bytes: &[i8] = unsafe { mem::transmute(unsigned_bytes) };
        Ok(signed_bytes)
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, w: impl Write) -> Result<()> {
        self.as_slice().encode(w)
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Vec<T> {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(len >= 0, "attempt to decode Vec with negative length");
        let len = len as usize;

        // Don't allocate more memory than what would roughly fit in a single packet in
        // case we get a malicious array length.
        let cap = (MAX_PACKET_SIZE as usize / mem::size_of::<T>().max(1)).min(len);
        let mut vec = Vec::with_capacity(cap);

        for _ in 0..len {
            vec.push(T::decode(r)?);
        }

        Ok(vec)
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Box<[T]> {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        Ok(Vec::decode(r)?.into_boxed_slice())
    }
}

// ==== String ==== //

impl Encode for str {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        let len = self.len();
        ensure!(
            len <= i32::MAX as usize,
            "byte length of string ({len}) exceeds i32::MAX"
        );

        VarInt(self.len() as i32).encode(&mut w)?;
        Ok(w.write_all(self.as_bytes())?)
    }
}

impl<'a> Decode<'a> for &'a str {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(len >= 0, "attempt to decode struct with negative length");
        let len = len as usize;
        ensure!(r.len() >= len, "not enough data remaining to decode string");

        let (res, remaining) = r.split_at(len);
        *r = remaining;

        Ok(std::str::from_utf8(res)?)
    }
}

impl Encode for String {
    fn encode(&self, w: impl Write) -> Result<()> {
        self.as_str().encode(w)
    }
}

impl Decode<'_> for String {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(<&str>::decode(r)?.into())
    }
}

impl Decode<'_> for Box<str> {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(<&str>::decode(r)?.into())
    }
}

// ==== Other ==== //

impl<T: Encode> Encode for Option<T> {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        match self {
            Some(t) => {
                true.encode(&mut w)?;
                t.encode(w)
            }
            None => false.encode(w),
        }
    }
}

impl<'a, T: Decode<'a>> Decode<'a> for Option<T> {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        Ok(match bool::decode(r)? {
            true => Some(T::decode(r)?),
            false => None,
        })
    }
}

impl<'a, B> Encode for Cow<'a, B>
where
    B: ToOwned + Encode,
{
    fn encode(&self, w: impl Write) -> Result<()> {
        self.as_ref().encode(w)
    }
}

impl<'a, B> Decode<'a> for Cow<'a, B>
where
    B: ToOwned,
    &'a B: Decode<'a>,
{
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        <&B>::decode(r).map(Cow::Borrowed)
    }
}

impl Encode for Uuid {
    fn encode(&self, w: impl Write) -> Result<()> {
        self.as_u128().encode(w)
    }
}

impl<'a> Decode<'a> for Uuid {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        u128::decode(r).map(Uuid::from_u128)
    }
}

impl Encode for Compound {
    fn encode(&self, w: impl Write) -> Result<()> {
        Ok(valence_nbt::to_binary_writer(w, self, "")?)
    }
}

impl Decode<'_> for Compound {
    fn decode(r: &mut &[u8]) -> Result<Self> {
        Ok(valence_nbt::from_binary_slice(r)?.0)
    }
}
