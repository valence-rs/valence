use std::io::Write;
use std::slice;

use byteorder::{BigEndian, WriteBytesExt};

use super::{modified_utf8, Error, Result};
use crate::tag::Tag;
use crate::{i8_slice_as_u8_slice, Compound, List, Value};

impl Compound {
    /// Encodes uncompressed NBT binary data to the provided writer.
    ///
    /// Only compounds are permitted at the top level. This is why the function
    /// accepts a [`Compound`] reference rather than a [`Value`].
    ///
    /// Additionally, the root compound can be given a name. Typically the empty
    /// string `""` is used.
    pub fn to_binary<W: Write>(&self, writer: W, root_name: &str) -> Result<()> {
        let mut state = EncodeState { writer };

        state.write_tag(Tag::Compound)?;
        state.write_string(root_name)?;
        state.write_compound(self)?;

        Ok(())
    }

    /// Returns the number of bytes that will be written when
    /// [`Compound::to_binary`] is called with this compound and root name.
    ///
    /// If `to_binary` results in `Ok`, the exact number of bytes
    /// reported by this function will have been written. If the result is
    /// `Err`, then the reported count will be greater than or equal to the
    /// number of bytes that have actually been written.
    pub fn written_size(&self, root_name: &str) -> usize {
        fn value_size(val: &Value) -> usize {
            match val {
                Value::Byte(_) => 1,
                Value::Short(_) => 2,
                Value::Int(_) => 4,
                Value::Long(_) => 8,
                Value::Float(_) => 4,
                Value::Double(_) => 8,
                Value::ByteArray(ba) => 4 + ba.len(),
                Value::String(s) => string_size(s),
                Value::List(l) => list_size(l),
                Value::Compound(c) => compound_size(c),
                Value::IntArray(ia) => 4 + ia.len() * 4,
                Value::LongArray(la) => 4 + la.len() * 8,
            }
        }

        fn list_size(l: &List) -> usize {
            let elems_size = match l {
                List::End => 0,
                List::Byte(b) => b.len(),
                List::Short(s) => s.len() * 2,
                List::Int(i) => i.len() * 4,
                List::Long(l) => l.len() * 8,
                List::Float(f) => f.len() * 4,
                List::Double(d) => d.len() * 8,
                List::ByteArray(ba) => ba.iter().map(|b| 4 + b.len()).sum(),
                List::String(s) => s.iter().map(|s| string_size(s)).sum(),
                List::List(l) => l.iter().map(list_size).sum(),
                List::Compound(c) => c.iter().map(compound_size).sum(),
                List::IntArray(i) => i.iter().map(|i| 4 + i.len() * 4).sum(),
                List::LongArray(l) => l.iter().map(|l| 4 + l.len() * 8).sum(),
            };

            1 + 4 + elems_size
        }

        fn string_size(s: &str) -> usize {
            2 + modified_utf8::encoded_len(s)
        }

        fn compound_size(c: &Compound) -> usize {
            c.iter()
                .map(|(k, v)| 1 + string_size(k) + value_size(v))
                .sum::<usize>()
                + 1
        }

        1 + string_size(root_name) + compound_size(self)
    }
}

struct EncodeState<W> {
    writer: W,
}

impl<W: Write> EncodeState<W> {
    fn write_tag(&mut self, tag: Tag) -> Result<()> {
        Ok(self.writer.write_u8(tag as u8)?)
    }

    fn write_value(&mut self, v: &Value) -> Result<()> {
        match v {
            Value::Byte(b) => self.write_byte(*b),
            Value::Short(s) => self.write_short(*s),
            Value::Int(i) => self.write_int(*i),
            Value::Long(l) => self.write_long(*l),
            Value::Float(f) => self.write_float(*f),
            Value::Double(d) => self.write_double(*d),
            Value::ByteArray(ba) => self.write_byte_array(ba),
            Value::String(s) => self.write_string(s),
            Value::List(l) => self.write_any_list(l),
            Value::Compound(c) => self.write_compound(c),
            Value::IntArray(ia) => self.write_int_array(ia),
            Value::LongArray(la) => self.write_long_array(la),
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

    fn write_string(&mut self, s: &str) -> Result<()> {
        let len = modified_utf8::encoded_len(s);

        match len.try_into() {
            Ok(n) => self.writer.write_u16::<BigEndian>(n)?,
            Err(_) => {
                return Err(Error::new_owned(format!(
                    "string of length {len} exceeds maximum of u16::MAX"
                )))
            }
        }

        // Conversion to modified UTF-8 always increases the size of the string.
        // If the new len is equal to the original len, we know it doesn't need
        // to be re-encoded.
        if len == s.len() {
            self.writer.write_all(s.as_bytes())?;
        } else {
            modified_utf8::write_modified_utf8(&mut self.writer, s)?;
        }

        Ok(())
    }

    fn write_any_list(&mut self, list: &List) -> Result<()> {
        match list {
            List::End => {
                self.write_tag(Tag::End)?;
                // Length
                self.writer.write_i32::<BigEndian>(0)?;
                Ok(())
            }
            List::Byte(bl) => {
                self.write_tag(Tag::Byte)?;

                match bl.len().try_into() {
                    Ok(len) => self.write_int(len)?,
                    Err(_) => {
                        return Err(Error::new_owned(format!(
                            "byte list of length {} exceeds maximum of i32::MAX",
                            bl.len(),
                        )))
                    }
                }

                Ok(self.writer.write_all(i8_slice_as_u8_slice(bl))?)
            }
            List::Short(sl) => self.write_list(sl, Tag::Short, |st, s| st.write_short(*s)),
            List::Int(il) => self.write_list(il, Tag::Int, |st, i| st.write_int(*i)),
            List::Long(ll) => self.write_list(ll, Tag::Long, |st, l| st.write_long(*l)),
            List::Float(fl) => self.write_list(fl, Tag::Float, |st, f| st.write_float(*f)),
            List::Double(dl) => self.write_list(dl, Tag::Double, |st, d| st.write_double(*d)),
            List::ByteArray(bal) => {
                self.write_list(bal, Tag::ByteArray, |st, ba| st.write_byte_array(ba))
            }
            List::String(sl) => self.write_list(sl, Tag::String, |st, s| st.write_string(s)),
            List::List(ll) => self.write_list(ll, Tag::List, |st, l| st.write_any_list(l)),
            List::Compound(cl) => self.write_list(cl, Tag::Compound, |st, c| st.write_compound(c)),
            List::IntArray(ial) => {
                self.write_list(ial, Tag::IntArray, |st, ia| st.write_int_array(ia))
            }
            List::LongArray(lal) => {
                self.write_list(lal, Tag::LongArray, |st, la| st.write_long_array(la))
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
                    "{elem_type} list of length {} exceeds maximum of i32::MAX",
                    list.len(),
                )))
            }
        }

        for elem in list {
            write_elem(self, elem)?;
        }

        Ok(())
    }

    fn write_compound(&mut self, c: &Compound) -> Result<()> {
        for (k, v) in c.iter() {
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
