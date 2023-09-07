use std::mem;

use byteorder::{BigEndian, ReadBytesExt};
use cesu8::Cesu8DecodingError;

use super::{Error, Result};
use crate::tag::Tag;
use crate::{Compound, List, Value};

/// Decodes uncompressed NBT binary data from the provided slice.
///
/// The string returned in the tuple is the name of the root compound
/// (typically the empty string).
pub fn from_binary(slice: &mut &[u8]) -> Result<(Compound, String)> {
    let mut state = DecodeState { slice, depth: 0 };

    let root_tag = state.read_tag()?;

    if root_tag != Tag::Compound {
        return Err(Error::new_owned(format!(
            "expected root tag for compound (got {})",
            root_tag.name(),
        )));
    }

    let root_name = state.read_string()?;
    let root = state.read_compound()?;

    debug_assert_eq!(state.depth, 0);

    Ok((root, root_name))
}

/// Maximum recursion depth to prevent overflowing the call stack.
const MAX_DEPTH: usize = 512;

struct DecodeState<'a, 'b> {
    slice: &'a mut &'b [u8],
    /// Current recursion depth.
    depth: usize,
}

impl DecodeState<'_, '_> {
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
        match self.slice.read_u8()? {
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

    fn read_value(&mut self, tag: Tag) -> Result<Value> {
        match tag {
            Tag::End => unreachable!("illegal TAG_End argument"),
            Tag::Byte => Ok(self.read_byte()?.into()),
            Tag::Short => Ok(self.read_short()?.into()),
            Tag::Int => Ok(self.read_int()?.into()),
            Tag::Long => Ok(self.read_long()?.into()),
            Tag::Float => Ok(self.read_float()?.into()),
            Tag::Double => Ok(self.read_double()?.into()),
            Tag::ByteArray => Ok(self.read_byte_array()?.into()),
            Tag::String => Ok(self.read_string()?.into()),
            Tag::List => self.check_depth(|st| Ok(st.read_any_list()?.into())),
            Tag::Compound => self.check_depth(|st| Ok(st.read_compound()?.into())),
            Tag::IntArray => Ok(self.read_int_array()?.into()),
            Tag::LongArray => Ok(self.read_long_array()?.into()),
        }
    }

    fn read_byte(&mut self) -> Result<i8> {
        Ok(self.slice.read_i8()?)
    }

    fn read_short(&mut self) -> Result<i16> {
        Ok(self.slice.read_i16::<BigEndian>()?)
    }

    fn read_int(&mut self) -> Result<i32> {
        Ok(self.slice.read_i32::<BigEndian>()?)
    }

    fn read_long(&mut self) -> Result<i64> {
        Ok(self.slice.read_i64::<BigEndian>()?)
    }

    fn read_float(&mut self) -> Result<f32> {
        Ok(self.slice.read_f32::<BigEndian>()?)
    }

    fn read_double(&mut self) -> Result<f64> {
        Ok(self.slice.read_f64::<BigEndian>()?)
    }

    fn read_byte_array(&mut self) -> Result<Vec<i8>> {
        let len = self.slice.read_i32::<BigEndian>()?;

        if len.is_negative() {
            return Err(Error::new_owned(format!(
                "negative byte array length of {len}"
            )));
        }

        if len as usize > self.slice.len() {
            return Err(Error::new_owned(format!(
                "byte array length of {len} exceeds remainder of input"
            )));
        }

        let (left, right) = self.slice.split_at(len as usize);

        let array = left.iter().map(|b| *b as i8).collect();
        *self.slice = right;

        Ok(array)
    }

    fn read_string(&mut self) -> Result<String> {
        let len = self.slice.read_u16::<BigEndian>()?.into();

        if len > self.slice.len() {
            return Err(Error::new_owned(format!(
                "string of length {len} exceeds remainder of input"
            )));
        }

        let (left, right) = self.slice.split_at(len);

        match cesu8::from_java_cesu8(left) {
            Ok(cow) => {
                *self.slice = right;
                Ok(cow.into())
            }
            Err(Cesu8DecodingError) => {
                Err(Error::new_static("could not convert CESU-8 data to UTF-8"))
            }
        }
    }

    fn read_any_list(&mut self) -> Result<List> {
        match self.read_tag()? {
            Tag::End => match self.read_int()? {
                0 => Ok(List::End),
                len => Err(Error::new_owned(format!(
                    "TAG_End list with nonzero length of {len}"
                ))),
            },
            Tag::Byte => Ok(self.read_list(Tag::Byte, 1, |st| st.read_byte())?.into()),
            Tag::Short => Ok(self.read_list(Tag::Short, 2, |st| st.read_short())?.into()),
            Tag::Int => Ok(self.read_list(Tag::Int, 4, |st| st.read_int())?.into()),
            Tag::Long => Ok(self.read_list(Tag::Long, 8, |st| st.read_long())?.into()),
            Tag::Float => Ok(self.read_list(Tag::Float, 4, |st| st.read_float())?.into()),
            Tag::Double => Ok(self
                .read_list(Tag::Double, 8, |st| st.read_double())?
                .into()),
            Tag::ByteArray => Ok(self
                .read_list(Tag::ByteArray, 0, |st| st.read_byte_array())?
                .into()),
            Tag::String => Ok(self
                .read_list(Tag::String, 0, |st| st.read_string())?
                .into()),
            Tag::List => self
                .check_depth(|st| Ok(st.read_list(Tag::List, 0, |st| st.read_any_list())?.into())),
            Tag::Compound => self.check_depth(|st| {
                Ok(st
                    .read_list(Tag::Compound, 0, |st| st.read_compound())?
                    .into())
            }),
            Tag::IntArray => Ok(self
                .read_list(Tag::IntArray, 0, |st| st.read_int_array())?
                .into()),
            Tag::LongArray => Ok(self
                .read_list(Tag::LongArray, 0, |st| st.read_long_array())?
                .into()),
        }
    }

    /// Assumes the element tag has already been read.
    ///
    /// `min_elem_size` is the minimum size of the list element when encoded.
    #[inline]
    fn read_list<T, F>(
        &mut self,
        elem_type: Tag,
        elem_size: usize,
        mut read_elem: F,
    ) -> Result<Vec<T>>
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

        // Ensure we don't reserve more than the maximum amount of memory required given
        // the size of the remaining input.
        if len as u64 * elem_size as u64 > self.slice.len() as u64 {
            return Err(Error::new_owned(format!(
                "{} list of length {len} exceeds remainder of input",
                elem_type.name()
            )));
        }

        let mut list = Vec::with_capacity(if elem_size == 0 { 0 } else { len as usize });

        for _ in 0..len {
            list.push(read_elem(self)?);
        }

        Ok(list)
    }

    fn read_compound(&mut self) -> Result<Compound> {
        let mut compound = Compound::new();

        loop {
            let tag = self.read_tag()?;
            if tag == Tag::End {
                return Ok(compound);
            }

            compound.insert(self.read_string()?, self.read_value(tag)?);
        }
    }

    fn read_int_array(&mut self) -> Result<Vec<i32>> {
        let len = self.read_int()?;

        if len.is_negative() {
            return Err(Error::new_owned(format!(
                "negative int array length of {len}",
            )));
        }

        if len as u64 * mem::size_of::<i32>() as u64 > self.slice.len() as u64 {
            return Err(Error::new_owned(format!(
                "int array of length {len} exceeds remainder of input"
            )));
        }

        let mut array = Vec::with_capacity(len as usize);
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

        if len as u64 * mem::size_of::<i64>() as u64 > self.slice.len() as u64 {
            return Err(Error::new_owned(format!(
                "long array of length {len} exceeds remainder of input"
            )));
        }

        let mut array = Vec::with_capacity(len as usize);
        for _ in 0..len {
            array.push(self.read_long()?);
        }

        Ok(array)
    }
}
