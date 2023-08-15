use std::marker::PhantomData;

use serde::ser::{Impossible, SerializeMap, SerializeSeq, SerializeStruct};
use serde::{Serialize, Serializer};

use super::Error;
use crate::{i8_slice_as_u8_slice, u8_vec_into_i8_vec, Compound, List, Value};

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Byte(v) => serializer.serialize_i8(*v),
            Value::Short(v) => serializer.serialize_i16(*v),
            Value::Int(v) => serializer.serialize_i32(*v),
            Value::Long(v) => serializer.serialize_i64(*v),
            Value::Float(v) => serializer.serialize_f32(*v),
            Value::Double(v) => serializer.serialize_f64(*v),
            Value::ByteArray(v) => serializer.serialize_bytes(i8_slice_as_u8_slice(v)),
            Value::String(v) => serializer.serialize_str(v),
            Value::List(v) => v.serialize(serializer),
            Value::Compound(v) => v.serialize(serializer),
            Value::IntArray(v) => v.serialize(serializer),
            Value::LongArray(v) => v.serialize(serializer),
        }
    }
}

impl Serialize for List {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            List::End => serializer.serialize_seq(Some(0))?.end(),
            List::Byte(v) => v.serialize(serializer),
            List::Short(v) => v.serialize(serializer),
            List::Int(v) => v.serialize(serializer),
            List::Long(v) => v.serialize(serializer),
            List::Float(v) => v.serialize(serializer),
            List::Double(v) => v.serialize(serializer),
            List::ByteArray(v) => v.serialize(serializer),
            List::String(v) => v.serialize(serializer),
            List::List(v) => v.serialize(serializer),
            List::Compound(v) => v.serialize(serializer),
            List::IntArray(v) => v.serialize(serializer),
            List::LongArray(v) => v.serialize(serializer),
        }
    }
}

macro_rules! unsupported {
    ($lit:literal) => {
        Err(Error::new(concat!("unsupported type: ", $lit)))
    };
}

/// [`Serializer`] whose output is [`Compound`].
pub struct CompoundSerializer;

impl Serializer for CompoundSerializer {
    type Ok = Compound;

    type Error = Error;

    type SerializeSeq = Impossible<Self::Ok, Self::Error>;

    type SerializeTuple = Impossible<Self::Ok, Self::Error>;

    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;

    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;

    type SerializeMap = GenericSerializeMap<Self::Ok>;

    type SerializeStruct = GenericSerializeStruct<Self::Ok>;

    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        unsupported!("bool")
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        unsupported!("i8")
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        unsupported!("i16")
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        unsupported!("i32")
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        unsupported!("i64")
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

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        unsupported!("f32")
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        unsupported!("f64")
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        unsupported!("char")
    }

    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        unsupported!("str")
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        unsupported!("bytes")
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        unsupported!("none")
    }

    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        unsupported!("some")
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        unsupported!("unit")
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
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        unsupported!("newtype variant")
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        unsupported!("seq")
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

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(GenericSerializeMap::new(len))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(GenericSerializeStruct::new(len))
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
}

/// [`Serializer`] whose output is [`Value`].
struct ValueSerializer;

impl Serializer for ValueSerializer {
    type Ok = Value;

    type Error = Error;

    type SerializeSeq = ValueSerializeSeq;

    type SerializeTuple = Impossible<Self::Ok, Self::Error>;

    type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;

    type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;

    type SerializeMap = GenericSerializeMap<Self::Ok>;

    type SerializeStruct = GenericSerializeStruct<Self::Ok>;

    type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Byte(v as _))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Byte(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Short(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Long(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Byte(v as _))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Short(v as _))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int(v as _))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Long(v as _))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Double(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(v.into()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(v.into()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Value::ByteArray(u8_vec_into_i8_vec(v.into())))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        unsupported!("none")
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        unsupported!("unit")
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(Value::String(variant.into()))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
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
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        unsupported!("newtype variant")
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ValueSerializeSeq::End {
            len: len.unwrap_or(0),
        })
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

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(GenericSerializeMap::new(len))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(GenericSerializeStruct::new(len))
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
}

enum ValueSerializeSeq {
    End { len: usize },
    Byte(Vec<i8>),
    Short(Vec<i16>),
    Int(Vec<i32>),
    Long(Vec<i64>),
    Float(Vec<f32>),
    Double(Vec<f64>),
    ByteArray(Vec<Vec<i8>>),
    String(Vec<String>),
    List(Vec<List>),
    Compound(Vec<Compound>),
    IntArray(Vec<Vec<i32>>),
    LongArray(Vec<Vec<i64>>),
}

impl SerializeSeq for ValueSerializeSeq {
    type Ok = Value;

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        macro_rules! serialize_variant {
            ($variant:ident, $vec:ident, $elem:ident) => {{
                match $elem.serialize(ValueSerializer)? {
                    Value::$variant(val) => {
                        $vec.push(val);
                        Ok(())
                    }
                    _ => Err(Error::new(concat!(
                        "heterogeneous NBT list (expected `",
                        stringify!($variant),
                        "` element)"
                    ))),
                }
            }};
        }

        match self {
            Self::End { len } => {
                fn vec<T>(elem: T, len: usize) -> Vec<T> {
                    let mut vec = Vec::with_capacity(len);
                    vec.push(elem);
                    vec
                }

                // Set the first element of the list.
                *self = match value.serialize(ValueSerializer)? {
                    Value::Byte(v) => Self::Byte(vec(v, *len)),
                    Value::Short(v) => Self::Short(vec(v, *len)),
                    Value::Int(v) => Self::Int(vec(v, *len)),
                    Value::Long(v) => Self::Long(vec(v, *len)),
                    Value::Float(v) => Self::Float(vec(v, *len)),
                    Value::Double(v) => Self::Double(vec(v, *len)),
                    Value::ByteArray(v) => Self::ByteArray(vec(v, *len)),
                    Value::String(v) => Self::String(vec(v, *len)),
                    Value::List(v) => Self::List(vec(v, *len)),
                    Value::Compound(v) => Self::Compound(vec(v, *len)),
                    Value::IntArray(v) => Self::IntArray(vec(v, *len)),
                    Value::LongArray(v) => Self::LongArray(vec(v, *len)),
                };
                Ok(())
            }
            Self::Byte(v) => serialize_variant!(Byte, v, value),
            Self::Short(v) => serialize_variant!(Short, v, value),
            Self::Int(v) => serialize_variant!(Int, v, value),
            Self::Long(v) => serialize_variant!(Long, v, value),
            Self::Float(v) => serialize_variant!(Float, v, value),
            Self::Double(v) => serialize_variant!(Double, v, value),
            Self::ByteArray(v) => serialize_variant!(ByteArray, v, value),
            Self::String(v) => serialize_variant!(String, v, value),
            Self::List(v) => serialize_variant!(List, v, value),
            Self::Compound(v) => serialize_variant!(Compound, v, value),
            Self::IntArray(v) => serialize_variant!(IntArray, v, value),
            Self::LongArray(v) => serialize_variant!(LongArray, v, value),
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(match self {
            Self::End { .. } => List::End.into(),
            Self::Byte(v) => v.into(),
            Self::Short(v) => List::Short(v).into(),
            Self::Int(v) => v.into(),
            Self::Long(v) => List::Long(v).into(),
            Self::Float(v) => List::Float(v).into(),
            Self::Double(v) => List::Double(v).into(),
            Self::ByteArray(v) => List::ByteArray(v).into(),
            Self::String(v) => List::String(v).into(),
            Self::List(v) => List::List(v).into(),
            Self::Compound(v) => List::Compound(v).into(),
            Self::IntArray(v) => List::IntArray(v).into(),
            Self::LongArray(v) => List::LongArray(v).into(),
        })
    }
}

#[doc(hidden)]
pub struct GenericSerializeMap<Ok> {
    /// Temp storage for `serialize_key`.
    key: Option<String>,
    res: Compound,
    _marker: PhantomData<Ok>,
}

impl<Ok> GenericSerializeMap<Ok> {
    pub fn new(len: Option<usize>) -> Self {
        Self {
            key: None,
            res: Compound::with_capacity(len.unwrap_or(0)),
            _marker: PhantomData,
        }
    }
}

impl<Ok> SerializeMap for GenericSerializeMap<Ok>
where
    Compound: Into<Ok>,
{
    type Ok = Ok;

    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        debug_assert!(
            self.key.is_none(),
            "call to `serialize_key` must be followed by `serialize_value`"
        );

        match key.serialize(ValueSerializer)? {
            Value::String(s) => {
                self.key = Some(s);
                Ok(())
            }
            _ => Err(Error::new("invalid map key type (expected string)")),
        }
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        let key = self
            .key
            .take()
            .expect("missing previous call to `serialize_key`");
        self.res.insert(key, value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.res.into())
    }
}

#[doc(hidden)]
pub struct GenericSerializeStruct<Ok> {
    c: Compound,
    _marker: PhantomData<Ok>,
}

impl<Ok> GenericSerializeStruct<Ok> {
    fn new(len: usize) -> Self {
        Self {
            c: Compound::with_capacity(len),
            _marker: PhantomData,
        }
    }
}

impl<Ok> SerializeStruct for GenericSerializeStruct<Ok>
where
    Compound: Into<Ok>,
{
    type Ok = Ok;

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.c.insert(key, value.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.c.into())
    }
}
