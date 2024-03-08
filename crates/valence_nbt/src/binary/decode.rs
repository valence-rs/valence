use std::borrow::Cow;
use std::hash::Hash;
use std::{fmt, io, mem};

use crate::conv::u8_slice_as_i8_slice;
use crate::tag::Tag;
use crate::{Compound, Error, List, Result, Value};

/// Decode an NBT value from the given buffer of bytes.
///
/// Returns both the root NBT value and the root name (typically the empty
/// string). If the root value is of type [`Tag::End`], then `None` is returned.
/// If the data is malformed or the reader returns an error, then an error is
/// returned.
pub fn from_binary<'a, S>(reader: impl ReadBytes<'a>) -> Result<Option<(S, Value<S>)>>
where
    S: FromModifiedUtf8<'a> + Hash + Ord,
{
    let mut state = DecodeState { reader, depth: 0 };

    let tag = state.read_tag()?;

    if tag == Tag::End {
        return Ok(None);
    }

    let name = state.read_string::<S>()?;
    let value = state.read_value::<S>(tag)?;

    debug_assert_eq!(state.depth, 0);

    Ok(Some((name, value)))
}

/// Maximum recursion depth to prevent overflowing the call stack.
const MAX_DEPTH: usize = 512;

struct DecodeState<R> {
    reader: R,
    /// Current recursion depth.
    depth: usize,
}

impl<'a, R: ReadBytes<'a>> DecodeState<R> {
    #[inline]
    fn check_depth<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        if self.depth >= MAX_DEPTH {
            return Err(Error::new_static("reached maximum recursion depth"));
        }

        self.depth += 1;
        let res = f(self);
        self.depth -= 1;
        res
    }

    fn read_tag(&mut self) -> Result<Tag> {
        match self.read_byte()? {
            0 => Ok(Tag::End),
            1 => Ok(Tag::Byte),
            2 => Ok(Tag::Short),
            3 => Ok(Tag::Int),
            4 => Ok(Tag::Long),
            5 => Ok(Tag::Float),
            6 => Ok(Tag::Double),
            7 => Ok(Tag::ByteArray),
            8 => Ok(Tag::String),
            9 => Ok(Tag::List),
            10 => Ok(Tag::Compound),
            11 => Ok(Tag::IntArray),
            12 => Ok(Tag::LongArray),
            byte => Err(Error::new_owned(format!("invalid tag byte of {byte:#x}"))),
        }
    }

    /// Read a value identified by the given tag.
    ///
    /// # Panics
    ///
    /// Panics if the tag is [`Tag::End`].
    #[track_caller]
    fn read_value<S>(&mut self, tag: Tag) -> Result<Value<S>>
    where
        S: FromModifiedUtf8<'a> + Hash + Ord,
    {
        Ok(match tag {
            Tag::End => panic!("cannot read value of Tag_END"),
            Tag::Byte => self.read_byte()?.into(),
            Tag::Short => self.read_short()?.into(),
            Tag::Int => self.read_int()?.into(),
            Tag::Long => self.read_long()?.into(),
            Tag::Float => self.read_float()?.into(),
            Tag::Double => self.read_double()?.into(),
            Tag::ByteArray => self.read_byte_array()?.into(),
            Tag::String => Value::String(self.read_string::<S>()?),
            Tag::List => self.check_depth(|st| st.read_any_list::<S>())?.into(),
            Tag::Compound => self.check_depth(|st| st.read_compound::<S>())?.into(),
            Tag::IntArray => self.read_int_array()?.into(),
            Tag::LongArray => self.read_long_array()?.into(),
        })
    }

    fn read_byte(&mut self) -> Result<i8> {
        Ok(self.reader.read_bytes(1)?[0] as i8)
    }

    fn read_short(&mut self) -> Result<i16> {
        Ok(i16::from_be_bytes(
            self.reader.read_bytes(2)?.try_into().unwrap(),
        ))
    }

    fn read_int(&mut self) -> Result<i32> {
        Ok(i32::from_be_bytes(
            self.reader.read_bytes(4)?.try_into().unwrap(),
        ))
    }

    fn read_long(&mut self) -> Result<i64> {
        Ok(i64::from_be_bytes(
            self.reader.read_bytes(8)?.try_into().unwrap(),
        ))
    }

    fn read_float(&mut self) -> Result<f32> {
        Ok(f32::from_be_bytes(
            self.reader.read_bytes(4)?.try_into().unwrap(),
        ))
    }

    fn read_double(&mut self) -> Result<f64> {
        Ok(f64::from_be_bytes(
            self.reader.read_bytes(8)?.try_into().unwrap(),
        ))
    }

    fn read_byte_array(&mut self) -> Result<Vec<i8>> {
        let len = self.read_int()?;

        if len.is_negative() {
            return Err(Error::new_owned(format!(
                "negative byte array length of {len}"
            )));
        }

        if len as usize > self.reader.remaining() {
            return Err(Error::new_owned(format!(
                "byte array length of {len} exceeds remainder of input"
            )));
        }

        let slice = u8_slice_as_i8_slice(self.reader.read_bytes(len as usize)?);

        debug_assert_eq!(slice.len(), len as usize);

        Ok(slice.into())
    }

    fn read_string<S>(&mut self) -> Result<S>
    where
        S: FromModifiedUtf8<'a>,
    {
        let len = self.read_short()? as usize;

        if len > self.reader.remaining() {
            return Err(Error::new_owned(format!(
                "string of length {len} exceeds remainder of input"
            )));
        }

        S::from_modified_utf8(self.reader.read_bytes(len)?)
            .map_err(|_| Error::new_static("could not decode modified UTF-8 data"))
    }

    fn read_any_list<S>(&mut self) -> Result<List<S>>
    where
        S: FromModifiedUtf8<'a> + Hash + Ord,
    {
        match self.read_tag()? {
            Tag::End => match self.read_int()? {
                0 => Ok(List::End),
                len => Err(Error::new_owned(format!(
                    "TAG_End list with nonzero length of {len}"
                ))),
            },
            Tag::Byte => Ok(self.read_list(Tag::Byte, |st| st.read_byte())?.into()),
            Tag::Short => Ok(self.read_list(Tag::Short, |st| st.read_short())?.into()),
            Tag::Int => Ok(self.read_list(Tag::Int, |st| st.read_int())?.into()),
            Tag::Long => Ok(self.read_list(Tag::Long, |st| st.read_long())?.into()),
            Tag::Float => Ok(self.read_list(Tag::Float, |st| st.read_float())?.into()),
            Tag::Double => Ok(self.read_list(Tag::Double, |st| st.read_double())?.into()),
            Tag::ByteArray => Ok(self
                .read_list(Tag::ByteArray, |st| st.read_byte_array())?
                .into()),
            Tag::String => Ok(List::String(
                self.read_list(Tag::String, |st| st.read_string::<S>())?,
            )),
            Tag::List => self.check_depth(|st| {
                Ok(st
                    .read_list(Tag::List, |st| st.read_any_list::<S>())?
                    .into())
            }),
            Tag::Compound => self.check_depth(|st| {
                Ok(st
                    .read_list(Tag::Compound, |st| st.read_compound::<S>())?
                    .into())
            }),
            Tag::IntArray => Ok(self
                .read_list(Tag::IntArray, |st| st.read_int_array())?
                .into()),
            Tag::LongArray => Ok(self
                .read_list(Tag::LongArray, |st| st.read_long_array())?
                .into()),
        }
    }

    /// Assumes the element tag has already been read.
    #[inline]
    fn read_list<T, F>(&mut self, elem_type: Tag, mut read_elem: F) -> Result<Vec<T>>
    where
        F: FnMut(&mut Self) -> Result<T>,
    {
        let len = self.read_int()?;

        if len.is_negative() {
            return Err(Error::new_owned(format!(
                "negative {} list length of {len}",
                elem_type.name()
            )));
        }

        let mut list = Vec::with_capacity(cautious_capacity::<T>(len as usize));

        for _ in 0..len {
            list.push(read_elem(self)?);
        }

        Ok(list)
    }

    fn read_compound<S>(&mut self) -> Result<Compound<S>>
    where
        S: FromModifiedUtf8<'a> + Hash + Ord,
    {
        let mut compound = Compound::new();

        loop {
            let tag = self.read_tag()?;
            if tag == Tag::End {
                return Ok(compound);
            }

            compound.insert(self.read_string::<S>()?, self.read_value::<S>(tag)?);
        }
    }

    fn read_int_array(&mut self) -> Result<Vec<i32>> {
        let len = self.read_int()?;

        if len.is_negative() {
            return Err(Error::new_owned(format!(
                "negative int array length of {len}",
            )));
        }

        if len as u64 * 4 > self.reader.remaining() as u64 {
            return Err(Error::new_owned(format!(
                "int array of length {len} exceeds remainder of input"
            )));
        }

        let mut array = Vec::with_capacity(len as usize);

        // TODO: SIMDify the endian swapping?
        for _ in 0..len {
            array.push(self.read_int()?);
        }

        Ok(array)
    }

    fn read_long_array(&mut self) -> Result<Vec<i64>> {
        let len = self.read_int()?;

        if len.is_negative() {
            return Err(Error::new_owned(format!(
                "negative long array length of {len}",
            )));
        }

        if len as u64 * 8 > self.reader.remaining() as u64 {
            return Err(Error::new_owned(format!(
                "long array of length {len} exceeds remainder of input"
            )));
        }

        let mut array = Vec::with_capacity(len as usize);

        // TODO: SIMDify the endian swapping?
        for _ in 0..len {
            array.push(self.read_long()?);
        }

        Ok(array)
    }
}

/// Prevents preallocating too much memory in case we get a malicious or invalid
/// sequence length.
fn cautious_capacity<Element>(size_hint: usize) -> usize {
    // TODO: How large can we make this?
    const MAX_PREALLOC_BYTES: usize = 2048;

    if mem::size_of::<Element>() == 0 {
        0
    } else {
        size_hint.min(MAX_PREALLOC_BYTES / mem::size_of::<Element>())
    }
}

pub trait ReadBytes<'a> {
    fn read_bytes(&mut self, count: usize) -> io::Result<&'a [u8]>;

    /// Returns the number of remaining bytes in the input.
    fn remaining(&self) -> usize;
}

impl<'a, T> ReadBytes<'a> for &mut T
where
    T: ReadBytes<'a>,
{
    fn read_bytes(&mut self, count: usize) -> io::Result<&'a [u8]> {
        (**self).read_bytes(count)
    }

    fn remaining(&self) -> usize {
        (**self).remaining()
    }
}

impl<'a> ReadBytes<'a> for &'a [u8] {
    fn read_bytes(&mut self, count: usize) -> io::Result<&'a [u8]> {
        if count > self.len() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        let (l, r) = self.split_at(count);
        *self = r;
        Ok(l)
    }

    fn remaining(&self) -> usize {
        self.len()
    }
}

impl<'a> ReadBytes<'a> for io::Cursor<&'a [u8]> {
    fn read_bytes(&mut self, count: usize) -> io::Result<&'a [u8]> {
        let remaining_slice =
            &self.get_ref()[self.position().min(self.get_ref().len() as u64) as usize..];

        if count > remaining_slice.len() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        self.set_position(self.position() + count as u64);

        Ok(&remaining_slice[..count])
    }

    fn remaining(&self) -> usize {
        self.get_ref().len() - self.position() as usize
    }
}

pub trait FromModifiedUtf8<'de>: Sized {
    fn from_modified_utf8(bytes: &'de [u8]) -> Result<Self, FromModifiedUtf8Error>;
}

impl<'a> FromModifiedUtf8<'a> for Cow<'a, str> {
    fn from_modified_utf8(bytes: &'a [u8]) -> Result<Self, FromModifiedUtf8Error> {
        cesu8::from_java_cesu8(bytes).map_err(move |_| FromModifiedUtf8Error)
    }
}

impl<'a> FromModifiedUtf8<'a> for String {
    fn from_modified_utf8(bytes: &'a [u8]) -> Result<Self, FromModifiedUtf8Error> {
        match cesu8::from_java_cesu8(bytes) {
            Ok(str) => Ok(str.into_owned()),
            Err(_) => Err(FromModifiedUtf8Error),
        }
    }
}

impl<'a> FromModifiedUtf8<'a> for Box<str> {
    fn from_modified_utf8(bytes: &'a [u8]) -> Result<Self, FromModifiedUtf8Error> {
        String::from_modified_utf8(bytes).map(|s| s.into())
    }
}

#[cfg(feature = "java_string")]
impl<'a> FromModifiedUtf8<'a> for Cow<'a, java_string::JavaStr> {
    fn from_modified_utf8(bytes: &'a [u8]) -> Result<Self, FromModifiedUtf8Error> {
        java_string::JavaStr::from_modified_utf8(bytes).map_err(|_| FromModifiedUtf8Error)
    }
}

#[cfg(feature = "java_string")]
impl<'a> FromModifiedUtf8<'a> for java_string::JavaString {
    fn from_modified_utf8(bytes: &'a [u8]) -> Result<Self, FromModifiedUtf8Error> {
        match java_string::JavaStr::from_modified_utf8(bytes) {
            Ok(str) => Ok(str.into_owned()),
            Err(_) => Err(FromModifiedUtf8Error),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FromModifiedUtf8Error;

impl fmt::Display for FromModifiedUtf8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("could not decode modified UTF-8 string")
    }
}

impl std::error::Error for FromModifiedUtf8Error {}
