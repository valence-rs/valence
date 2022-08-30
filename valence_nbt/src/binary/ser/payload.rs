use std::io::Write;

use byteorder::{BigEndian, WriteBytesExt};
use serde::{Serialize, Serializer};

use crate::binary::ser::map::SerializeMap;
use crate::binary::ser::seq::SerializeSeq;
use crate::binary::ser::structs::SerializeStruct;
use crate::binary::ser::{write_string, Impossible};
use crate::{ArrayType, Error, Tag};

pub struct PayloadSerializer<'w, 'n, W: ?Sized> {
    writer: &'w mut W,
    state: State<'n>,
}

#[derive(Clone, Copy)]
enum State<'n> {
    Named(&'n str),
    FirstListElement { len: i32, written_tag: Tag },
    SeqElement { element_type: Tag },
    Array(ArrayType),
}

impl<'w, 'n, W: Write + ?Sized> PayloadSerializer<'w, 'n, W> {
    pub(super) fn named(writer: &'w mut W, name: &'n str) -> Self {
        Self {
            writer,
            state: State::Named(name),
        }
    }

    pub(super) fn first_list_element(writer: &'w mut W, len: i32) -> Self {
        Self {
            writer,
            state: State::FirstListElement {
                len,
                written_tag: Tag::End,
            },
        }
    }

    pub(super) fn seq_element(writer: &'w mut W, element_type: Tag) -> Self {
        Self {
            writer,
            state: State::SeqElement { element_type },
        }
    }

    pub(super) fn written_tag(&self) -> Option<Tag> {
        match self.state {
            State::FirstListElement { written_tag, .. } if written_tag != Tag::End => {
                Some(written_tag)
            }
            _ => None,
        }
    }

    fn check_state(&mut self, tag: Tag) -> Result<(), Error> {
        match &mut self.state {
            State::Named(name) => {
                self.writer.write_u8(tag as u8)?;
                write_string(&mut *self.writer, *name)?;
            }
            State::FirstListElement { len, written_tag } => {
                self.writer.write_u8(tag as u8)?;
                self.writer.write_i32::<BigEndian>(*len)?;
                *written_tag = tag;
            }
            State::SeqElement { element_type } => {
                if tag != *element_type {
                    return Err(Error::new_owned(format!(
                        "list/array elements must be homogeneous (got {tag}, expected \
                         {element_type})"
                    )));
                }
            }
            State::Array(array_type) => {
                let msg = match array_type {
                    ArrayType::Byte => "a byte array",
                    ArrayType::Int => "an int array",
                    ArrayType::Long => "a long array",
                };

                return Err(Error::new_owned(format!(
                    "expected a seq for {msg}, got {tag} instead"
                )));
            }
        }

        Ok(())
    }
}

macro_rules! unsupported {
    ($typ:literal) => {
        Err(Error::new_static(concat!($typ, " is not supported")))
    };
}

impl<'a, W: Write + ?Sized> Serializer for &'a mut PayloadSerializer<'_, '_, W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerializeSeq<'a, W>;
    type SerializeTuple = Impossible;
    type SerializeTupleStruct = Impossible;
    type SerializeTupleVariant = Impossible;
    type SerializeMap = SerializeMap<'a, W>;
    type SerializeStruct = SerializeStruct<'a, W>;
    type SerializeStructVariant = Impossible;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.check_state(Tag::Byte)?;
        Ok(self.writer.write_i8(v as i8)?)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.check_state(Tag::Byte)?;
        Ok(self.writer.write_i8(v)?)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.check_state(Tag::Short)?;
        Ok(self.writer.write_i16::<BigEndian>(v)?)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.check_state(Tag::Int)?;
        Ok(self.writer.write_i32::<BigEndian>(v)?)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        self.check_state(Tag::Long)?;
        Ok(self.writer.write_i64::<BigEndian>(v)?)
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        unsupported!("u8")
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        unsupported!("u16")
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        unsupported!("u32")
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        unsupported!("u64")
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.check_state(Tag::Float)?;
        Ok(self.writer.write_f32::<BigEndian>(v)?)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        self.check_state(Tag::Double)?;
        Ok(self.writer.write_f64::<BigEndian>(v)?)
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        unsupported!("char")
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.check_state(Tag::String)?;
        write_string(&mut *self.writer, v)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        unsupported!("&[u8]")
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        unsupported!("()")
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        unsupported!("unit struct")
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        unsupported!("unit variant")
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        unsupported!("newtype struct")
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        let (array_tag, array_type) = match (name, variant) {
            (crate::ARRAY_ENUM_NAME, crate::BYTE_ARRAY_VARIANT_NAME) => {
                (Tag::ByteArray, ArrayType::Byte)
            }
            (crate::ARRAY_ENUM_NAME, crate::INT_ARRAY_VARIANT_NAME) => {
                (Tag::IntArray, ArrayType::Int)
            }
            (crate::ARRAY_ENUM_NAME, crate::LONG_ARRAY_VARIANT_NAME) => {
                (Tag::LongArray, ArrayType::Long)
            }
            _ => return unsupported!("newtype variant"),
        };

        self.check_state(array_tag)?;

        value.serialize(&mut PayloadSerializer {
            writer: self.writer,
            state: State::Array(array_type),
        })
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        if let State::Array(array_type) = self.state {
            let len = match len {
                Some(len) => len,
                None => return Err(Error::new_static("array length must be known up front")),
            };

            match len.try_into() {
                Ok(len) => {
                    self.writer.write_i32::<BigEndian>(len)?;
                    Ok(SerializeSeq::array(
                        self.writer,
                        array_type.element_tag(),
                        len,
                    ))
                }
                Err(_) => Err(Error::new_static("length of array exceeds i32::MAX")),
            }
        } else {
            self.check_state(Tag::List)?;

            let len = match len {
                Some(len) => len,
                None => return Err(Error::new_static("list length must be known up front")),
            };

            match len.try_into() {
                Ok(len) => Ok(SerializeSeq::list(self.writer, len)),
                Err(_) => Err(Error::new_static("length of list exceeds i32::MAX")),
            }
        }
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        unsupported!("tuple")
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        unsupported!("tuple struct")
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unsupported!("tuple variant")
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.check_state(Tag::Compound)?;

        Ok(SerializeMap {
            writer: self.writer,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.check_state(Tag::Compound)?;

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
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        unsupported!("struct variant")
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}
