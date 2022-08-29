use std::borrow::Cow;
use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};
use cesu8::from_java_cesu8;
use serde::de::Visitor;
use serde::{de, forward_to_deserialize_any};
use smallvec::SmallVec;

use crate::binary::de::array::EnumAccess;
use crate::binary::de::compound::MapAccess;
use crate::binary::de::list::SeqAccess;
use crate::{ArrayType, Error, Tag, CESU8_DECODE_ERROR};

pub(super) struct PayloadDeserializer<'w, R: ?Sized> {
    pub reader: &'w mut R,
    /// The type of payload to be deserialized.
    pub tag: Tag,
}

impl<'de: 'w, 'w, R: Read + ?Sized> de::Deserializer<'de> for PayloadDeserializer<'w, R> {
    type Error = Error;

    forward_to_deserialize_any! {
        i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.tag {
            Tag::End => unreachable!("invalid payload tag"),
            Tag::Byte => visitor.visit_i8(self.reader.read_i8()?),
            Tag::Short => visitor.visit_i16(self.reader.read_i16::<BigEndian>()?),
            Tag::Int => visitor.visit_i32(self.reader.read_i32::<BigEndian>()?),
            Tag::Long => visitor.visit_i64(self.reader.read_i64::<BigEndian>()?),
            Tag::Float => visitor.visit_f32(self.reader.read_f32::<BigEndian>()?),
            Tag::Double => visitor.visit_f64(self.reader.read_f64::<BigEndian>()?),
            Tag::ByteArray => visitor.visit_enum(EnumAccess {
                reader: self.reader,
                array_type: ArrayType::Byte,
            }),
            Tag::String => {
                let mut buf = SmallVec::<[u8; 128]>::new();
                for _ in 0..self.reader.read_u16::<BigEndian>()? {
                    buf.push(self.reader.read_u8()?);
                }

                match from_java_cesu8(&buf).map_err(|_| Error::new_static(CESU8_DECODE_ERROR))? {
                    Cow::Borrowed(s) => visitor.visit_str(s),
                    Cow::Owned(string) => visitor.visit_string(string),
                }
            }
            Tag::List => {
                let element_tag = Tag::from_u8(self.reader.read_u8()?)?;
                let len = self.reader.read_i32::<BigEndian>()?;

                if len < 0 {
                    return Err(Error::new_static("list with negative length"));
                }

                if element_tag == Tag::End && len != 0 {
                    return Err(Error::new_static(
                        "list with TAG_End element type must have length zero",
                    ));
                }

                visitor.visit_seq(SeqAccess {
                    reader: self.reader,
                    element_tag,
                    remaining: len as u32,
                })
            }
            Tag::Compound => visitor.visit_map(MapAccess::new(self.reader, &[])),
            Tag::IntArray => visitor.visit_enum(EnumAccess {
                reader: self.reader,
                array_type: ArrayType::Int,
            }),
            Tag::LongArray => visitor.visit_enum(EnumAccess {
                reader: self.reader,
                array_type: ArrayType::Long,
            }),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.tag == Tag::Byte {
            match self.reader.read_i8()? {
                0 => visitor.visit_bool(false),
                1 => visitor.visit_bool(true),
                n => visitor.visit_i8(n),
            }
        } else {
            self.deserialize_any(visitor)
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.tag == Tag::Compound {
            visitor.visit_map(MapAccess::new(self.reader, fields))
        } else {
            self.deserialize_any(visitor)
        }
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}
