use std::io::Write;
use std::result::Result as StdResult;

use anyhow::anyhow;
use byteorder::{BigEndian, WriteBytesExt};
use cesu8::to_java_cesu8;
use serde::{ser, Serialize};

use crate::{Error, Result, Tag};

pub fn to_writer<W, T>(mut writer: W, root_name: &str, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize + ?Sized,
{
    value.serialize(&mut Serializer::new(&mut writer, root_name))
}

pub struct Serializer<'w, 'n, W: ?Sized> {
    writer: &'w mut W,
    allowed_tag: AllowedTag,
    ser_state: SerState<'n>,
}

#[derive(Copy, Clone)]
enum AllowedTag {
    /// Any tag type is permitted to be serialized.
    Any {
        /// Set to the type that was serialized. Is unspecified if serialization
        /// failed or has not taken place yet.
        written_tag: Tag,
    },
    /// Only one specific tag type is permitted to be serialized.
    One {
        /// The permitted tag.
        tag: Tag,
        /// The error message if a tag mismatch happens.
        errmsg: &'static str,
    },
}

enum SerState<'n> {
    /// Serialize just the payload and nothing else.
    PayloadOnly,
    /// Prefix the payload with the tag and a length.
    /// Used for the first element of lists.
    FirstListElement {
        /// Length of the list being serialized.
        len: i32,
    },
    /// Prefix the payload with the tag and a name.
    /// Used for compound fields and the root compound.
    Named { name: &'n str },
}

impl<'w, 'n, W: Write + ?Sized> Serializer<'w, 'n, W> {
    pub fn new(writer: &'w mut W, root_name: &'n str) -> Self {
        Self {
            writer,
            allowed_tag: AllowedTag::One {
                tag: Tag::Compound,
                errmsg: "root value must be a compound",
            },
            ser_state: SerState::Named { name: root_name },
        }
    }

    pub fn writer(&mut self) -> &mut W {
        self.writer
    }

    pub fn root_name(&self) -> &'n str {
        match &self.ser_state {
            SerState::Named { name } => *name,
            _ => unreachable!(),
        }
    }

    pub fn set_root_name(&mut self, root_name: &'n str) {
        self.ser_state = SerState::Named { name: root_name };
    }

    fn write_header(&mut self, tag: Tag) -> Result<()> {
        match &mut self.allowed_tag {
            AllowedTag::Any { written_tag } => *written_tag = tag,
            AllowedTag::One {
                tag: expected_tag,
                errmsg,
            } => {
                if tag != *expected_tag {
                    let e = anyhow!(*errmsg).context(format!(
                        "attempt to serialize {tag} where {expected_tag} was expected"
                    ));
                    return Err(Error(e));
                }
            }
        }

        match &mut self.ser_state {
            SerState::PayloadOnly => {}
            SerState::FirstListElement { len } => {
                self.writer.write_u8(tag as u8)?;
                self.writer.write_i32::<BigEndian>(*len)?;
            }
            SerState::Named { name } => {
                self.writer.write_u8(tag as u8)?;
                write_string_payload(*name, self.writer)?;
            }
        }

        Ok(())
    }
}

type Impossible = ser::Impossible<(), Error>;

#[inline]
fn unsupported<T>(typ: &str) -> Result<T> {
    Err(Error(anyhow!("{typ} is not supported")))
}

impl<'a, W: Write + ?Sized> ser::Serializer for &'a mut Serializer<'_, '_, W> {
    type Error = Error;
    type Ok = ();
    type SerializeMap = SerializeMap<'a, W>;
    type SerializeSeq = SerializeSeq<'a, W>;
    type SerializeStruct = SerializeStruct<'a, W>;
    type SerializeStructVariant = Impossible;
    type SerializeTuple = Impossible;
    type SerializeTupleStruct = SerializeArray<'a, W>;
    type SerializeTupleVariant = Impossible;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.write_header(Tag::Byte)?;
        Ok(self.writer.write_i8(v as i8)?)
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.write_header(Tag::Byte)?;
        Ok(self.writer.write_i8(v)?)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.write_header(Tag::Short)?;
        Ok(self.writer.write_i16::<BigEndian>(v)?)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.write_header(Tag::Int)?;
        Ok(self.writer.write_i32::<BigEndian>(v)?)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.write_header(Tag::Long)?;
        Ok(self.writer.write_i64::<BigEndian>(v)?)
    }

    fn serialize_u8(self, _v: u8) -> Result<()> {
        unsupported("u8")
    }

    fn serialize_u16(self, _v: u16) -> Result<()> {
        unsupported("u16")
    }

    fn serialize_u32(self, _v: u32) -> Result<()> {
        unsupported("u32")
    }

    fn serialize_u64(self, _v: u64) -> Result<()> {
        unsupported("u64")
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.write_header(Tag::Float)?;
        Ok(self.writer.write_f32::<BigEndian>(v)?)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.write_header(Tag::Double)?;
        Ok(self.writer.write_f64::<BigEndian>(v)?)
    }

    fn serialize_char(self, _v: char) -> Result<()> {
        unsupported("char")
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.write_header(Tag::String)?;
        write_string_payload(v, self.writer)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        unsupported("&[u8]")
    }

    fn serialize_none(self) -> Result<()> {
        unsupported("Option<T>")
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        unsupported("Option<T>")
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        match variant_index.try_into() {
            Ok(idx) => self.serialize_i32(idx),
            Err(_) => Err(Error(anyhow!(
                "variant index of {variant_index} is out of range"
            ))),
        }
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: Serialize,
    {
        unsupported("newtype variant")
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.write_header(Tag::List)?;

        let len = match len {
            Some(len) => len,
            None => return Err(Error(anyhow!("list length must be known up front"))),
        };

        match len.try_into() {
            Ok(len) => Ok(SerializeSeq {
                writer: self.writer,
                element_type: Tag::End,
                remaining: len,
            }),
            Err(_) => Err(Error(anyhow!("length of list exceeds i32::MAX"))),
        }
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        unsupported("tuple")
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        let element_type = match name {
            crate::BYTE_ARRAY_MAGIC => {
                self.write_header(Tag::ByteArray)?;
                Tag::Byte
            }
            crate::INT_ARRAY_MAGIC => {
                self.write_header(Tag::IntArray)?;
                Tag::Int
            }
            crate::LONG_ARRAY_MAGIC => {
                self.write_header(Tag::LongArray)?;
                Tag::Long
            }
            _ => return unsupported("tuple struct"),
        };

        match len.try_into() {
            Ok(len) => {
                self.writer.write_i32::<BigEndian>(len)?;
                Ok(SerializeArray {
                    writer: self.writer,
                    element_type,
                    remaining: len,
                })
            }
            Err(_) => Err(Error(anyhow!("array length of {len} exceeds i32::MAX"))),
        }
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        unsupported("tuple variant")
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.write_header(Tag::Compound)?;

        Ok(SerializeMap {
            writer: self.writer,
        })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.write_header(Tag::Compound)?;

        Ok(SerializeStruct {
            writer: self.writer,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        unsupported("struct variant")
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

#[doc(hidden)]
pub struct SerializeSeq<'w, W: ?Sized> {
    writer: &'w mut W,
    /// The element type of this list. TAG_End if unknown.
    element_type: Tag,
    /// Number of elements left to serialize.
    remaining: i32,
}

impl<W: Write + ?Sized> ser::SerializeSeq for SerializeSeq<'_, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        if self.remaining <= 0 {
            return Err(Error(anyhow!(
                "attempt to serialize more list elements than specified"
            )));
        }

        if self.element_type == Tag::End {
            let mut ser = Serializer {
                writer: self.writer,
                allowed_tag: AllowedTag::Any {
                    written_tag: Tag::End,
                },
                ser_state: SerState::FirstListElement {
                    len: self.remaining,
                },
            };

            value.serialize(&mut ser)?;

            self.element_type = match ser.allowed_tag {
                AllowedTag::Any { written_tag } => written_tag,
                AllowedTag::One { .. } => unreachable!(),
            };
        } else {
            value.serialize(&mut Serializer {
                writer: self.writer,
                allowed_tag: AllowedTag::One {
                    tag: self.element_type,
                    errmsg: "list elements must be homogeneous",
                },
                ser_state: SerState::PayloadOnly,
            })?;
        }

        self.remaining -= 1;

        Ok(())
    }

    fn end(self) -> Result<()> {
        if self.remaining > 0 {
            return Err(Error(anyhow!(
                "{} list element(s) left to serialize",
                self.remaining
            )));
        }

        // Were any elements written?
        if self.element_type == Tag::End {
            self.writer.write_u8(Tag::End as u8)?;
            // List length.
            self.writer.write_i32::<BigEndian>(0)?;
        }

        Ok(())
    }
}

#[doc(hidden)]
pub struct SerializeArray<'w, W: ?Sized> {
    writer: &'w mut W,
    element_type: Tag,
    remaining: i32,
}

impl<W: Write + ?Sized> ser::SerializeTupleStruct for SerializeArray<'_, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        if self.remaining <= 0 {
            return Err(Error(anyhow!(
                "attempt to serialize more array elements than specified"
            )));
        }

        value.serialize(&mut Serializer {
            writer: self.writer,
            allowed_tag: AllowedTag::One {
                tag: self.element_type,
                errmsg: "mismatched array element type",
            },
            ser_state: SerState::PayloadOnly,
        })?;

        self.remaining -= 1;

        Ok(())
    }

    fn end(self) -> Result<()> {
        if self.remaining > 0 {
            return Err(Error(anyhow!(
                "{} array element(s) left to serialize",
                self.remaining
            )));
        }

        Ok(())
    }
}

#[doc(hidden)]
pub struct SerializeMap<'w, W: ?Sized> {
    writer: &'w mut W,
}

impl<W: Write + ?Sized> ser::SerializeMap for SerializeMap<'_, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<()>
    where
        T: Serialize,
    {
        Err(Error(anyhow!("map keys cannot be serialized individually")))
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        Err(Error(anyhow!(
            "map values cannot be serialized individually"
        )))
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(&mut self, key: &K, value: &V) -> Result<()>
    where
        K: Serialize,
        V: Serialize,
    {
        key.serialize(MapEntrySerializer {
            writer: self.writer,
            value,
        })
    }

    fn end(self) -> Result<()> {
        Ok(self.writer.write_u8(Tag::End as u8)?)
    }
}

struct MapEntrySerializer<'w, 'v, W: ?Sized, V: ?Sized> {
    writer: &'w mut W,
    value: &'v V,
}

fn key_not_a_string<T>() -> Result<T> {
    Err(Error(anyhow!("map keys must be strings")))
}

impl<W, V> ser::Serializer for MapEntrySerializer<'_, '_, W, V>
where
    W: Write + ?Sized,
    V: Serialize + ?Sized,
{
    type Error = Error;
    type Ok = ();
    type SerializeMap = Impossible;
    type SerializeSeq = Impossible;
    type SerializeStruct = Impossible;
    type SerializeStructVariant = Impossible;
    type SerializeTuple = Impossible;
    type SerializeTupleStruct = Impossible;
    type SerializeTupleVariant = Impossible;

    fn serialize_bool(self, _v: bool) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_i8(self, _v: i8) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_i16(self, _v: i16) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_i32(self, _v: i32) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_i64(self, _v: i64) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_u8(self, _v: u8) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_u16(self, _v: u16) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_u32(self, _v: u32) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_u64(self, _v: u64) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_f32(self, _v: f32) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_f64(self, _v: f64) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_char(self, _v: char) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.value
            .serialize(&mut Serializer {
                writer: self.writer,
                allowed_tag: AllowedTag::Any {
                    written_tag: Tag::End,
                },
                ser_state: SerState::Named { name: v },
            })
            .map_err(|e| e.context(format!("map key `{v}`")))
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_none(self) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        key_not_a_string()
    }

    fn serialize_unit(self) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        key_not_a_string()
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, _value: &T) -> Result<()>
    where
        T: Serialize,
    {
        key_not_a_string()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: Serialize,
    {
        key_not_a_string()
    }

    fn serialize_seq(self, _len: Option<usize>) -> StdResult<Self::SerializeSeq, Self::Error> {
        key_not_a_string()
    }

    fn serialize_tuple(self, _len: usize) -> StdResult<Self::SerializeTuple, Self::Error> {
        key_not_a_string()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> StdResult<Self::SerializeTupleStruct, Self::Error> {
        key_not_a_string()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> StdResult<Self::SerializeTupleVariant, Self::Error> {
        key_not_a_string()
    }

    fn serialize_map(self, _len: Option<usize>) -> StdResult<Self::SerializeMap, Self::Error> {
        key_not_a_string()
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> StdResult<Self::SerializeStruct, Self::Error> {
        key_not_a_string()
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> StdResult<Self::SerializeStructVariant, Self::Error> {
        key_not_a_string()
    }
}

#[doc(hidden)]
pub struct SerializeStruct<'w, W: ?Sized> {
    writer: &'w mut W,
}

impl<W: Write + ?Sized> ser::SerializeStruct for SerializeStruct<'_, W> {
    type Error = Error;
    type Ok = ();

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value
            .serialize(&mut Serializer {
                writer: self.writer,
                allowed_tag: AllowedTag::Any {
                    written_tag: Tag::End,
                },
                ser_state: SerState::Named { name: key },
            })
            .map_err(|e| e.context(format!("field `{key}`")))
    }

    fn end(self) -> Result<()> {
        Ok(self.writer.write_u8(Tag::End as u8)?)
    }
}

fn write_string_payload(string: &str, writer: &mut (impl Write + ?Sized)) -> Result<()> {
    let data = to_java_cesu8(string);
    match data.len().try_into() {
        Ok(len) => writer.write_u16::<BigEndian>(len)?,
        Err(_) => return Err(Error(anyhow!("string byte length exceeds u16::MAX"))),
    };

    writer.write_all(&data)?;
    Ok(())
}
