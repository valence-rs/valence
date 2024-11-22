use std::borrow::Cow;
use std::hash::Hash;
use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};

use super::modified_utf8;
use crate::conv::i8_slice_as_u8_slice;
use crate::tag::Tag;
use crate::{Compound, Error, List, Result, Value};

/// Encodes uncompressed NBT binary data to the provided writer.
///
/// Only compounds are permitted at the top level. This is why the function
/// accepts a [`Compound`] reference rather than a [`Value`].
///
/// Additionally, the root compound can be given a name. Typically the empty
/// string `""` is used.
pub fn to_binary<W, S, R>(comp: &Compound<S>, writer: W, root_name: Option<&R>) -> Result<()>
where
    W: Write,
    S: ToModifiedUtf8 + Hash + Ord,
    R: ToModifiedUtf8 + ?Sized,
{
    let mut state = EncodeState { writer };

    state.write_tag(Tag::Compound)?;
    if let Some(root_name) = root_name {
        state.write_string(root_name)?;
    }
    state.write_compound(comp)?;

    Ok(())
}

/// Returns the number of bytes that will be written when
/// [`to_binary`] is called with this compound and root name.
///
/// If `to_binary` results in `Ok`, the exact number of bytes
/// reported by this function will have been written. If the result is
/// `Err`, then the reported count will be greater than or equal to the
/// number of bytes that have actually been written.
pub fn written_size<S, R>(comp: &Compound<S>, root_name: &R) -> usize
where
    S: ToModifiedUtf8 + Hash + Ord,
    R: ToModifiedUtf8 + ?Sized,
{
    fn value_size<S>(val: &Value<S>) -> usize
    where
        S: ToModifiedUtf8 + Hash + Ord,
    {
        match val {
            Value::Byte(_) => 1,
            Value::Short(_) => 2,
            Value::Int(_) => 4,
            Value::Long(_) => 8,
            Value::Float(_) => 4,
            Value::Double(_) => 8,
            Value::ByteArray(v) => 4 + v.len(),
            Value::String(v) => string_size(v),
            Value::List(v) => list_size(v),
            Value::Compound(v) => compound_size(v),
            Value::IntArray(v) => 4 + v.len() * 4,
            Value::LongArray(v) => 4 + v.len() * 8,
        }
    }

    fn list_size<S>(l: &List<S>) -> usize
    where
        S: ToModifiedUtf8 + Hash + Ord,
    {
        let elems_size = match l {
            List::End => 0,
            List::Byte(v) => v.len(),
            List::Short(v) => v.len() * 2,
            List::Int(v) => v.len() * 4,
            List::Long(v) => v.len() * 8,
            List::Float(v) => v.len() * 4,
            List::Double(v) => v.len() * 8,
            List::ByteArray(v) => v.iter().map(|b| 4 + b.len()).sum(),
            List::String(v) => v.iter().map(|s| string_size(s)).sum(),
            List::List(v) => v.iter().map(list_size).sum(),
            List::Compound(v) => v.iter().map(compound_size).sum(),
            List::IntArray(v) => v.iter().map(|i| 4 + i.len() * 4).sum(),
            List::LongArray(v) => v.iter().map(|l| 4 + l.len() * 8).sum(),
        };

        1 + 4 + elems_size
    }

    fn string_size<S: ToModifiedUtf8 + ?Sized>(s: &S) -> usize {
        2 + s.modified_uf8_len()
    }

    fn compound_size<S>(c: &Compound<S>) -> usize
    where
        S: ToModifiedUtf8 + Hash + Ord,
    {
        c.iter()
            .map(|(k, v)| 1 + string_size(k) + value_size(v))
            .sum::<usize>()
            + 1
    }

    1 + string_size(root_name) + compound_size(comp)
}

struct EncodeState<W> {
    writer: W,
}

impl<W: Write> EncodeState<W> {
    fn write_tag(&mut self, tag: Tag) -> Result<()> {
        Ok(self.writer.write_u8(tag as u8)?)
    }

    fn write_value<S>(&mut self, v: &Value<S>) -> Result<()>
    where
        S: ToModifiedUtf8 + Hash + Ord,
    {
        match v {
            Value::Byte(v) => self.write_byte(*v),
            Value::Short(v) => self.write_short(*v),
            Value::Int(v) => self.write_int(*v),
            Value::Long(v) => self.write_long(*v),
            Value::Float(v) => self.write_float(*v),
            Value::Double(v) => self.write_double(*v),
            Value::ByteArray(v) => self.write_byte_array(v),
            Value::String(v) => self.write_string(v),
            Value::List(v) => self.write_any_list(v),
            Value::Compound(v) => self.write_compound(v),
            Value::IntArray(v) => self.write_int_array(v),
            Value::LongArray(v) => self.write_long_array(v),
        }
    }

    fn write_byte(&mut self, byte: i8) -> Result<()> {
        Ok(self.writer.write_i8(byte)?)
    }

    fn write_short(&mut self, short: i16) -> Result<()> {
        Ok(self.writer.write_i16::<BigEndian>(short)?)
    }

    fn write_int(&mut self, int: i32) -> Result<()> {
        Ok(self.writer.write_i32::<BigEndian>(int)?)
    }

    fn write_long(&mut self, long: i64) -> Result<()> {
        Ok(self.writer.write_i64::<BigEndian>(long)?)
    }

    fn write_float(&mut self, float: f32) -> Result<()> {
        Ok(self.writer.write_f32::<BigEndian>(float)?)
    }

    fn write_double(&mut self, double: f64) -> Result<()> {
        Ok(self.writer.write_f64::<BigEndian>(double)?)
    }

    fn write_byte_array(&mut self, bytes: &[i8]) -> Result<()> {
        match bytes.len().try_into() {
            Ok(len) => self.write_int(len)?,
            Err(_) => {
                return Err(Error::new_owned(format!(
                    "byte array of length {} exceeds maximum of i32::MAX",
                    bytes.len(),
                )))
            }
        }

        Ok(self.writer.write_all(i8_slice_as_u8_slice(bytes))?)
    }

    fn write_string<S: ToModifiedUtf8 + ?Sized>(&mut self, s: &S) -> Result<()> {
        let len = s.modified_uf8_len();

        match len.try_into() {
            Ok(n) => self.writer.write_u16::<BigEndian>(n)?,
            Err(_) => {
                return Err(Error::new_owned(format!(
                    "string of length {len} exceeds maximum of u16::MAX"
                )))
            }
        }

        s.to_modified_utf8(len, &mut self.writer)?;

        Ok(())
    }

    fn write_any_list<S>(&mut self, list: &List<S>) -> Result<()>
    where
        S: ToModifiedUtf8 + Hash + Ord,
    {
        match list {
            List::End => {
                self.write_tag(Tag::End)?;
                // Length
                self.writer.write_i32::<BigEndian>(0)?;
                Ok(())
            }
            List::Byte(v) => {
                self.write_tag(Tag::Byte)?;

                match v.len().try_into() {
                    Ok(len) => self.write_int(len)?,
                    Err(_) => {
                        return Err(Error::new_owned(format!(
                            "byte list of length {} exceeds maximum of i32::MAX",
                            v.len(),
                        )))
                    }
                }

                Ok(self.writer.write_all(i8_slice_as_u8_slice(v))?)
            }
            List::Short(sl) => self.write_list(sl, Tag::Short, |st, v| st.write_short(*v)),
            List::Int(il) => self.write_list(il, Tag::Int, |st, v| st.write_int(*v)),
            List::Long(ll) => self.write_list(ll, Tag::Long, |st, v| st.write_long(*v)),
            List::Float(fl) => self.write_list(fl, Tag::Float, |st, v| st.write_float(*v)),
            List::Double(dl) => self.write_list(dl, Tag::Double, |st, v| st.write_double(*v)),
            List::ByteArray(v) => {
                self.write_list(v, Tag::ByteArray, |st, v| st.write_byte_array(v))
            }
            List::String(v) => self.write_list(v, Tag::String, |st, v| st.write_string(v)),
            List::List(v) => self.write_list(v, Tag::List, |st, v| st.write_any_list(v)),
            List::Compound(v) => self.write_list(v, Tag::Compound, |st, v| st.write_compound(v)),
            List::IntArray(v) => self.write_list(v, Tag::IntArray, |st, v| st.write_int_array(v)),
            List::LongArray(v) => {
                self.write_list(v, Tag::LongArray, |st, v| st.write_long_array(v))
            }
        }
    }

    fn write_list<T, F>(&mut self, list: &[T], elem_type: Tag, mut write_elem: F) -> Result<()>
    where
        F: FnMut(&mut Self, &T) -> Result<()>,
    {
        self.write_tag(elem_type)?;

        match list.len().try_into() {
            Ok(len) => self.writer.write_i32::<BigEndian>(len)?,
            Err(_) => {
                return Err(Error::new_owned(format!(
                    "{} list of length {} exceeds maximum of i32::MAX",
                    list.len(),
                    elem_type.name()
                )))
            }
        }

        for elem in list {
            write_elem(self, elem)?;
        }

        Ok(())
    }

    fn write_compound<S>(&mut self, c: &Compound<S>) -> Result<()>
    where
        S: ToModifiedUtf8 + Hash + Ord,
    {
        for (k, v) in c {
            self.write_tag(v.tag())?;
            self.write_string(k)?;
            self.write_value(v)?;
        }
        self.write_tag(Tag::End)?;

        Ok(())
    }

    fn write_int_array(&mut self, ia: &[i32]) -> Result<()> {
        match ia.len().try_into() {
            Ok(len) => self.write_int(len)?,
            Err(_) => {
                return Err(Error::new_owned(format!(
                    "int array of length {} exceeds maximum of i32::MAX",
                    ia.len(),
                )))
            }
        }

        for i in ia {
            self.write_int(*i)?;
        }

        Ok(())
    }

    fn write_long_array(&mut self, la: &[i64]) -> Result<()> {
        match la.len().try_into() {
            Ok(len) => self.write_int(len)?,
            Err(_) => {
                return Err(Error::new_owned(format!(
                    "long array of length {} exceeds maximum of i32::MAX",
                    la.len(),
                )))
            }
        }

        for l in la {
            self.write_long(*l)?;
        }

        Ok(())
    }
}

/// A string type which can be encoded into Java's [modified UTF-8](https://docs.oracle.com/javase/8/docs/api/java/io/DataInput.html#modified-utf-8).
pub trait ToModifiedUtf8 {
    fn modified_uf8_len(&self) -> usize;
    fn to_modified_utf8<W: Write>(&self, encoded_len: usize, writer: W) -> std::io::Result<()>;
}

impl ToModifiedUtf8 for str {
    fn modified_uf8_len(&self) -> usize {
        modified_utf8::encoded_len(self.as_bytes())
    }

    fn to_modified_utf8<W: Write>(&self, encoded_len: usize, mut writer: W) -> std::io::Result<()> {
        // Conversion to modified UTF-8 always increases the size of the string.
        // If the new len is equal to the original len, we know it doesn't need
        // to be re-encoded.
        if self.len() == encoded_len {
            writer.write_all(self.as_bytes())
        } else {
            modified_utf8::write_modified_utf8(writer, self)
        }
    }
}

impl ToModifiedUtf8 for Cow<'_, str> {
    #[inline]
    fn modified_uf8_len(&self) -> usize {
        str::modified_uf8_len(self)
    }

    fn to_modified_utf8<W: Write>(&self, encoded_len: usize, writer: W) -> std::io::Result<()> {
        str::to_modified_utf8(self, encoded_len, writer)
    }
}

impl ToModifiedUtf8 for String {
    #[inline]
    fn modified_uf8_len(&self) -> usize {
        str::modified_uf8_len(self)
    }

    fn to_modified_utf8<W: Write>(&self, encoded_len: usize, writer: W) -> std::io::Result<()> {
        str::to_modified_utf8(self, encoded_len, writer)
    }
}

#[cfg(feature = "java_string")]
impl ToModifiedUtf8 for java_string::JavaStr {
    fn modified_uf8_len(&self) -> usize {
        modified_utf8::encoded_len(self.as_bytes())
    }

    fn to_modified_utf8<W: Write>(
        &self,
        _encoded_len: usize,
        mut writer: W,
    ) -> std::io::Result<()> {
        writer.write_all(&self.to_modified_utf8())
    }
}

#[cfg(feature = "java_string")]
impl ToModifiedUtf8 for Cow<'_, java_string::JavaStr> {
    #[inline]
    fn modified_uf8_len(&self) -> usize {
        java_string::JavaStr::modified_uf8_len(self)
    }

    fn to_modified_utf8<W: Write>(&self, encoded_len: usize, writer: W) -> std::io::Result<()> {
        <java_string::JavaStr as ToModifiedUtf8>::to_modified_utf8(self, encoded_len, writer)
    }
}

#[cfg(feature = "java_string")]
impl ToModifiedUtf8 for java_string::JavaString {
    #[inline]
    fn modified_uf8_len(&self) -> usize {
        java_string::JavaStr::modified_uf8_len(self)
    }

    fn to_modified_utf8<W: Write>(&self, encoded_len: usize, writer: W) -> std::io::Result<()> {
        <java_string::JavaStr as ToModifiedUtf8>::to_modified_utf8(self, encoded_len, writer)
    }
}
