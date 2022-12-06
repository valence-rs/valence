use std::io::Write;
use std::marker::PhantomData;
use std::mem;

use anyhow::anyhow;

use crate::{Decode, Encode, Result};

/// Contains both a value of `T` and an [`EncodedBuf<T>`] to ensure the
/// buffer is updated when `T` is modified[^note].
///
/// The `Encode` implementation for `Cached<T>` encodes only the contained
/// `EncodedBuf`.
///
/// Use this type when you want `Encode` to be cached but you also need to read
/// from the value of `T`. If the value of `T` is write-only, consider using
/// `EncodedBuf` instead.
///
/// [`EncodedBuf<T>`]: EncodedBuf
/// [^note]: Assuming `T` does not use internal mutability.
#[derive(Debug)]
pub struct Cached<T> {
    val: T,
    buf: EncodedBuf<T>,
}

impl<T: Encode> Cached<T> {
    pub fn new(val: T) -> Self {
        let buf = EncodedBuf::new(&val);

        Self { val, buf }
    }

    pub fn get(&self) -> &T {
        &self.val
    }

    pub fn buf(&self) -> &EncodedBuf<T> {
        &self.buf
    }

    /// Provides a mutable reference to the contained `T` for modification. The
    /// buffer is re-encoded when the closure returns.
    pub fn modify<U>(&mut self, f: impl FnOnce(&mut T) -> U) -> U {
        let u = f(&mut self.val);
        self.buf.set(&self.val);
        u
    }

    pub fn replace(&mut self, new: T) -> T {
        self.modify(|old| mem::replace(old, new))
    }

    pub fn into_inner(self) -> (T, EncodedBuf<T>) {
        (self.val, self.buf)
    }
}

impl<T> Encode for Cached<T>
where
    T: Encode,
{
    fn encode(&self, w: impl Write) -> Result<()> {
        self.buf.encode(w)
    }
}

/// The `Decode` implementation for `Cached` exists for the sake of
/// completeness, but you probably shouldn't need to use it.
impl<'a, T> Decode<'a> for Cached<T>
where
    T: Encode + Decode<'a>,
{
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        let val = T::decode(r)?;
        Ok(Self::new(val))
    }
}

/// Caches the result of `T`'s [`Encode`] implementation into an owned buffer.
///
/// This is useful for types with expensive [`Encode`] implementations such as
/// [`Text`] or [`Compound`]. It has little to no benefit for primitive types
/// such as `i32`, `VarInt`, `&str`, `&[u8]`, etc.
///
/// # Examples
///
/// ```
/// use valence_protocol::{Encode, EncodedBuf};
///
/// let mut buf1 = Vec::new();
/// let mut buf2 = Vec::new();
///
/// "hello".encode(&mut buf1).unwrap();
///
/// let cache = EncodedBuf::new("hello");
/// cache.encode(&mut buf2).unwrap();
///
/// assert_eq!(buf1, buf2);
/// ```
///
/// [`Text`]: crate::text::Text
/// [`Compound`]: valence_nbt::Compound
#[derive(Debug)]
pub struct EncodedBuf<T: ?Sized> {
    buf: Vec<u8>,
    res: Result<()>,
    _marker: PhantomData<fn(T) -> T>,
}

impl<T: Encode + ?Sized> EncodedBuf<T> {
    pub fn new(t: &T) -> Self {
        let mut buf = Vec::new();
        let res = t.encode(&mut buf);

        Self {
            buf,
            res,
            _marker: PhantomData,
        }
    }

    pub fn set(&mut self, t: &T) {
        self.buf.clear();
        self.res = t.encode(&mut self.buf);
    }

    pub fn into_inner(self) -> Result<Vec<u8>> {
        self.res.map(|()| self.buf)
    }
}

impl<T: ?Sized> Encode for EncodedBuf<T> {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        match &self.res {
            Ok(()) => Ok(w.write_all(&self.buf)?),
            Err(e) => Err(anyhow!("{e:#}")),
        }
    }

    fn encoded_len(&self) -> usize {
        self.buf.len()
    }
}
