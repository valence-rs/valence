use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};
use zerocopy::AsBytes;

use crate::tag::Tag;
use crate::{Compound, Error, List, Result, Value, MAX_DEPTH};

/// Encodes uncompressed NBT binary data to the provided writer.
///
/// Only compounds are permitted at the top level. This is why the function
/// accepts a [`Compound`] reference rather than a [`Value`].
///
/// Additionally, the root compound can be given a name. Typically the empty
/// string `""` is used.
pub fn to_binary_writer<W: Write>(writer: W, compound: &Compound, root_name: &str) -> Result<()> {
    let mut state = EncodeState { writer, depth: 0 };

    state.write_tag(Tag::Compound)?;
    state.write_string(root_name)?;
    state.write_compound(compound)?;

    debug_assert_eq!(state.depth, 0);
    Ok(())
}

struct EncodeState<W> {
    writer: W,
    /// Current recursion depth.
    depth: usize,
}

impl<W: Write> EncodeState<W> {
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
            Value::List(l) => self.check_depth(|st| st.write_any_list(l)),
            Value::Compound(c) => self.check_depth(|st| st.write_compound(c)),
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

        Ok(self.writer.write_all(bytes.as_bytes())?)
    }

    fn write_string(&mut self, s: &str) -> Result<()> {
        let s = cesu8::to_java_cesu8(s);

        match s.len().try_into() {
            Ok(len) => self.writer.write_u16::<BigEndian>(len)?,
            Err(_) => {
                return Err(Error::new_owned(format!(
                    "string of length {} exceeds maximum of u16::MAX",
                    s.len()
                )))
            }
        }

        Ok(self.writer.write_all(&s)?)
    }

    fn write_any_list(&mut self, list: &List) -> Result<()> {
        match list {
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

                Ok(self.writer.write_all(bl.as_bytes())?)
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
            List::List(ll) => {
                self.check_depth(|st| st.write_list(ll, Tag::List, |st, l| st.write_any_list(l)))
            }
            List::Compound(cl) => self
                .check_depth(|st| st.write_list(cl, Tag::Compound, |st, c| st.write_compound(c))),
            List::IntArray(ial) => {
                self.write_list(ial, Tag::IntArray, |st, ia| st.write_int_array(ia))
            }
            List::LongArray(lal) => {
                self.write_list(lal, Tag::LongArray, |st, la| st.write_long_array(la))
            }
        }
    }

    fn write_list<T, F>(&mut self, list: &Vec<T>, elem_type: Tag, mut write_elem: F) -> Result<()>
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
            self.write_tag(Tag::element_type(v))?;
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
