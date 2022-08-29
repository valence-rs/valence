use std::io::Write;

use byteorder::WriteBytesExt;
use serde::{Serialize, Serializer};

use crate::binary::ser::map::SerializeMap;
use crate::binary::ser::structs::SerializeStruct;
use crate::binary::ser::{write_string, Impossible};
use crate::{Error, Tag};

#[non_exhaustive]
pub struct RootSerializer<'n, W> {
    pub writer: W,
    pub root_name: &'n str,
}

impl<'n, W: Write> RootSerializer<'n, W> {
    pub fn new(writer: W, root_name: &'n str) -> Self {
        Self { writer, root_name }
    }

    fn write_header(&mut self) -> Result<(), Error> {
        self.writer.write_u8(Tag::Compound as u8)?;
        write_string(&mut self.writer, self.root_name)
    }
}

macro_rules! not_compound {
    ($typ:literal) => {
        Err(Error::new_static(concat!(
            "root value must be a map or struct (got ",
            $typ,
            ")"
        )))
    };
}

impl<'a, W: Write> Serializer for &'a mut RootSerializer<'_, W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Impossible;
    type SerializeTuple = Impossible;
    type SerializeTupleStruct = Impossible;
    type SerializeTupleVariant = Impossible;
    type SerializeMap = SerializeMap<'a, W>;
    type SerializeStruct = SerializeStruct<'a, W>;
    type SerializeStructVariant = Impossible;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        not_compound!("bool")
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        not_compound!("i8")
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        not_compound!("i16")
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        not_compound!("i32")
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        not_compound!("i64")
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        not_compound!("u8")
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        not_compound!("u16")
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        not_compound!("u32")
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        not_compound!("u64")
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        not_compound!("f32")
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        not_compound!("f64")
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        not_compound!("char")
    }

    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        not_compound!("str")
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        not_compound!("&[u8]")
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        not_compound!("None")
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        not_compound!("Some")
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        not_compound!("()")
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        not_compound!("unit struct")
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        not_compound!("unit variant")
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        not_compound!("newtype struct")
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        not_compound!("newtype variant")
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        not_compound!("seq")
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        not_compound!("tuple")
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        not_compound!("tuple struct")
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        not_compound!("tuple variant")
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.write_header()?;

        Ok(SerializeMap {
            writer: &mut self.writer,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.write_header()?;

        Ok(SerializeStruct {
            writer: &mut self.writer,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        not_compound!("struct variant")
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}
