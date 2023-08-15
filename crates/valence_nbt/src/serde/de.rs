use std::fmt;

use serde::de::value::{MapAccessDeserializer, MapDeserializer, SeqAccessDeserializer};
use serde::de::{self, IntoDeserializer, SeqAccess, Visitor};
use serde::{forward_to_deserialize_any, Deserialize, Deserializer};

use super::Error;
use crate::{i8_vec_into_u8_vec, u8_slice_as_i8_slice, u8_vec_into_i8_vec, Compound, List, Value};

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a valid NBT type")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Byte(v as _))
            }

            fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Byte(v))
            }

            fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Short(v))
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Int(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Long(v))
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Byte(v as _))
            }

            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Short(v as _))
            }

            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Int(v as _))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Long(v as _))
            }

            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Float(v))
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Double(v))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::String(v.into()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::String(v))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::ByteArray(u8_slice_as_i8_slice(v).into()))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Value::ByteArray(u8_vec_into_i8_vec(v)))
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                Ok(List::deserialize(SeqAccessDeserializer::new(seq))?.into())
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                Ok(Compound::deserialize(MapAccessDeserializer::new(map))?.into())
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl<'de> Deserialize<'de> for List {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ListVisitor;

        impl<'de> Visitor<'de> for ListVisitor {
            type Value = List;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a sequence or bytes")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                match seq.next_element::<Value>()? {
                    Some(v) => match v {
                        Value::Byte(v) => deserialize_seq_remainder(v, seq),
                        Value::Short(v) => deserialize_seq_remainder(v, seq),
                        Value::Int(v) => deserialize_seq_remainder(v, seq),
                        Value::Long(v) => deserialize_seq_remainder(v, seq),
                        Value::Float(v) => deserialize_seq_remainder(v, seq),
                        Value::Double(v) => deserialize_seq_remainder(v, seq),
                        Value::ByteArray(v) => deserialize_seq_remainder(v, seq),
                        Value::String(v) => deserialize_seq_remainder(v, seq),
                        Value::List(v) => deserialize_seq_remainder(v, seq),
                        Value::Compound(v) => deserialize_seq_remainder(v, seq),
                        Value::IntArray(v) => deserialize_seq_remainder(v, seq),
                        Value::LongArray(v) => deserialize_seq_remainder(v, seq),
                    },
                    None => Ok(List::End),
                }
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(List::Byte(u8_vec_into_i8_vec(v)))
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(List::Byte(u8_slice_as_i8_slice(v).into()))
            }
        }

        deserializer.deserialize_seq(ListVisitor)
    }
}

/// Deserializes the remainder of a sequence after having
/// determined the type of the first element.
fn deserialize_seq_remainder<'de, T, A, R>(first: T, mut seq: A) -> Result<R, A::Error>
where
    T: Deserialize<'de>,
    Vec<T>: Into<R>,
    A: de::SeqAccess<'de>,
{
    let mut vec = match seq.size_hint() {
        Some(n) => Vec::with_capacity(n + 1),
        None => Vec::new(),
    };

    vec.push(first);

    while let Some(v) = seq.next_element()? {
        vec.push(v);
    }

    Ok(vec.into())
}

impl<'de> Deserializer<'de> for Compound {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(MapDeserializer::new(self.into_iter()))
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

impl<'de> IntoDeserializer<'de, Error> for Compound {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> Deserializer<'de> for Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Byte(v) => visitor.visit_i8(v),
            Value::Short(v) => visitor.visit_i16(v),
            Value::Int(v) => visitor.visit_i32(v),
            Value::Long(v) => visitor.visit_i64(v),
            Value::Float(v) => visitor.visit_f32(v),
            Value::Double(v) => visitor.visit_f64(v),
            Value::ByteArray(v) => visitor.visit_byte_buf(i8_vec_into_u8_vec(v)),
            Value::String(v) => visitor.visit_string(v),
            Value::List(v) => v.deserialize_any(visitor),
            Value::Compound(v) => v.into_deserializer().deserialize_any(visitor),
            Value::IntArray(v) => v.into_deserializer().deserialize_any(visitor),
            Value::LongArray(v) => v.into_deserializer().deserialize_any(visitor),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Byte(b) => visitor.visit_bool(b != 0),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::String(s) => visitor.visit_enum(s.into_deserializer()), // Unit variant.
            other => other.deserialize_any(visitor),
        }
    }

    forward_to_deserialize_any! {
        i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct identifier ignored_any
    }
}

impl<'de> IntoDeserializer<'de, Error> for Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'de> Deserializer<'de> for List {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        struct EndSeqAccess;

        impl<'de> SeqAccess<'de> for EndSeqAccess {
            type Error = Error;

            fn next_element_seed<T>(&mut self, _seed: T) -> Result<Option<T::Value>, Self::Error>
            where
                T: de::DeserializeSeed<'de>,
            {
                Ok(None)
            }
        }

        match self {
            List::End => visitor.visit_seq(EndSeqAccess),
            List::Byte(v) => visitor.visit_byte_buf(i8_vec_into_u8_vec(v)),
            List::Short(v) => v.into_deserializer().deserialize_any(visitor),
            List::Int(v) => v.into_deserializer().deserialize_any(visitor),
            List::Long(v) => v.into_deserializer().deserialize_any(visitor),
            List::Float(v) => v.into_deserializer().deserialize_any(visitor),
            List::Double(v) => v.into_deserializer().deserialize_any(visitor),
            List::ByteArray(v) => v.into_deserializer().deserialize_any(visitor),
            List::String(v) => v.into_deserializer().deserialize_any(visitor),
            List::List(v) => v.into_deserializer().deserialize_any(visitor),
            List::Compound(v) => v.into_deserializer().deserialize_any(visitor),
            List::IntArray(v) => v.into_deserializer().deserialize_any(visitor),
            List::LongArray(v) => v.into_deserializer().deserialize_any(visitor),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

impl<'de> IntoDeserializer<'de, Error> for List {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}
