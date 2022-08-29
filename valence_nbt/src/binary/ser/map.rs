use std::io::Write;

use anyhow::anyhow;
use byteorder::WriteBytesExt;
use serde::{ser, Serialize, Serializer};

use crate::binary::ser::payload::PayloadSerializer;
use crate::binary::ser::Impossible;
use crate::{Error, Tag};

pub struct SerializeMap<'w, W: ?Sized> {
    pub(super) writer: &'w mut W,
}

impl<'w, W: Write + ?Sized> ser::SerializeMap for SerializeMap<'w, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, _key: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        Err(Error(anyhow!("map keys cannot be serialized individually")))
    }

    fn serialize_value<T: ?Sized>(&mut self, _value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        Err(Error(anyhow!(
            "map values cannot be serialized individually"
        )))
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(
        &mut self,
        key: &K,
        value: &V,
    ) -> Result<(), Self::Error>
    where
        K: Serialize,
        V: Serialize,
    {
        key.serialize(MapEntrySerializer {
            writer: self.writer,
            value,
        })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer.write_u8(Tag::End as u8)?)
    }
}

struct MapEntrySerializer<'w, 'v, W: ?Sized, V: ?Sized> {
    writer: &'w mut W,
    value: &'v V,
}

fn key_not_a_string<T>(typ: &str) -> Result<T, Error> {
    Err(Error(anyhow!("map keys must be strings (got {typ})")))
}

impl<W: Write + ?Sized, V: Serialize + ?Sized> Serializer for MapEntrySerializer<'_, '_, W, V> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Impossible;
    type SerializeTuple = Impossible;
    type SerializeTupleStruct = Impossible;
    type SerializeTupleVariant = Impossible;
    type SerializeMap = Impossible;
    type SerializeStruct = Impossible;
    type SerializeStructVariant = Impossible;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("bool")
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("i8")
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("i16")
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("i32")
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("i64")
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("u8")
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("u16")
    }

    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("u32")
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("u64")
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("f32")
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("f64")
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("char")
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.value
            .serialize(&mut PayloadSerializer::named(self.writer, v))
            .map_err(|e| e.context(format!("key `{v}`")))
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("&[u8]")
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("None")
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        key_not_a_string("Some")
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("()")
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("unit struct")
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        key_not_a_string("unit variant")
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        key_not_a_string("newtype struct")
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
        key_not_a_string("newtype variant")
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        key_not_a_string("seq")
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        key_not_a_string("tuple")
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        key_not_a_string("tuple struct")
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        key_not_a_string("tuple variant")
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        key_not_a_string("map")
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        key_not_a_string("struct")
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        key_not_a_string("struct variant")
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}
