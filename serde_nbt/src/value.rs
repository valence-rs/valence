use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;

use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{byte_array, int_array, long_array};

/// Represents an arbitrary NBT value.
#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(List),
    Compound(Compound),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

pub type Compound = HashMap<String, Value>;

/// An NBT list value.
///
/// NBT lists are homogeneous, meaning each list element must be of the same
/// type. This is opposed to a format like JSON where lists can be
/// heterogeneous:
///
/// ```json
/// [42, "hello", {}]
/// ```
///
/// Every possible element type has its own variant in this enum. As a result,
/// heterogeneous lists are unrepresentable.
#[derive(Clone, PartialEq, Debug)]
pub enum List {
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

impl List {
    pub fn len(&self) -> usize {
        match self {
            List::Byte(l) => l.len(),
            List::Short(l) => l.len(),
            List::Int(l) => l.len(),
            List::Long(l) => l.len(),
            List::Float(l) => l.len(),
            List::Double(l) => l.len(),
            List::ByteArray(l) => l.len(),
            List::String(l) => l.len(),
            List::List(l) => l.len(),
            List::Compound(l) => l.len(),
            List::IntArray(l) => l.len(),
            List::LongArray(l) => l.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Self::Byte(v)
    }
}

impl From<i16> for Value {
    fn from(v: i16) -> Self {
        Self::Short(v)
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Self::Int(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Self::Long(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Self::Float(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Self::Double(v)
    }
}

impl From<Vec<i8>> for Value {
    fn from(v: Vec<i8>) -> Self {
        Self::ByteArray(v)
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl<'a> From<&'a str> for Value {
    fn from(v: &'a str) -> Self {
        Self::String(v.to_owned())
    }
}

impl<'a> From<Cow<'a, str>> for Value {
    fn from(v: Cow<'a, str>) -> Self {
        Self::String(v.into_owned())
    }
}

impl From<List> for Value {
    fn from(v: List) -> Self {
        Self::List(v)
    }
}

impl From<Compound> for Value {
    fn from(v: Compound) -> Self {
        Self::Compound(v)
    }
}

impl From<Vec<i32>> for Value {
    fn from(v: Vec<i32>) -> Self {
        Self::IntArray(v)
    }
}

impl From<Vec<i64>> for Value {
    fn from(v: Vec<i64>) -> Self {
        Self::LongArray(v)
    }
}

impl From<Vec<i8>> for List {
    fn from(v: Vec<i8>) -> Self {
        List::Byte(v)
    }
}

impl From<Vec<i16>> for List {
    fn from(v: Vec<i16>) -> Self {
        List::Short(v)
    }
}

impl From<Vec<i32>> for List {
    fn from(v: Vec<i32>) -> Self {
        List::Int(v)
    }
}

impl From<Vec<i64>> for List {
    fn from(v: Vec<i64>) -> Self {
        List::Long(v)
    }
}

impl From<Vec<f32>> for List {
    fn from(v: Vec<f32>) -> Self {
        List::Float(v)
    }
}

impl From<Vec<f64>> for List {
    fn from(v: Vec<f64>) -> Self {
        List::Double(v)
    }
}

impl From<Vec<Vec<i8>>> for List {
    fn from(v: Vec<Vec<i8>>) -> Self {
        List::ByteArray(v)
    }
}

impl From<Vec<String>> for List {
    fn from(v: Vec<String>) -> Self {
        List::String(v)
    }
}

impl From<Vec<List>> for List {
    fn from(v: Vec<List>) -> Self {
        List::List(v)
    }
}

impl From<Vec<Compound>> for List {
    fn from(v: Vec<Compound>) -> Self {
        List::Compound(v)
    }
}

impl From<Vec<Vec<i32>>> for List {
    fn from(v: Vec<Vec<i32>>) -> Self {
        List::IntArray(v)
    }
}

impl From<Vec<Vec<i64>>> for List {
    fn from(v: Vec<Vec<i64>>) -> Self {
        List::LongArray(v)
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Byte(v) => v.serialize(serializer),
            Value::Short(v) => v.serialize(serializer),
            Value::Int(v) => v.serialize(serializer),
            Value::Long(v) => v.serialize(serializer),
            Value::Float(v) => v.serialize(serializer),
            Value::Double(v) => v.serialize(serializer),
            Value::ByteArray(v) => byte_array(v, serializer),
            Value::String(v) => v.serialize(serializer),
            Value::List(v) => v.serialize(serializer),
            Value::Compound(v) => v.serialize(serializer),
            Value::IntArray(v) => int_array(v, serializer),
            Value::LongArray(v) => long_array(v, serializer),
        }
    }
}

impl Serialize for List {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            List::Byte(l) => l.serialize(serializer),
            List::Short(l) => l.serialize(serializer),
            List::Int(l) => l.serialize(serializer),
            List::Long(l) => l.serialize(serializer),
            List::Float(l) => l.serialize(serializer),
            List::Double(l) => l.serialize(serializer),
            List::ByteArray(l) => l.serialize(serializer),
            List::String(l) => l.serialize(serializer),
            List::List(l) => l.serialize(serializer),
            List::Compound(l) => l.serialize(serializer),
            List::IntArray(l) => l.serialize(serializer),
            List::LongArray(l) => l.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "a representable NBT value")
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Byte(v))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Short(v))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Int(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Long(v))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Float(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::Double(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::String(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(Value::String(v.to_owned()))
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        ListVisitor.visit_seq(seq).map(Value::List)
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        visit_map(map).map(Value::Compound)
    }
}

impl<'de> Deserialize<'de> for List {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(ListVisitor)
    }
}

struct ListVisitor;

impl<'de> Visitor<'de> for ListVisitor {
    type Value = List;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "an NBT list")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut list = List::Byte(Vec::new());

        while seq
            .next_element_seed(DeserializeListElement(&mut list))?
            .is_some()
        {}

        Ok(list)
    }
}

struct DeserializeListElement<'a>(&'a mut List);

impl<'de, 'a> DeserializeSeed<'de> for DeserializeListElement<'a> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

macro_rules! visit {
    ($self:expr, $variant:ident, $value:expr, $error:ty) => {
        if $self.0.is_empty() {
            *$self.0 = List::$variant(vec![$value]);
            Ok(())
        } else if let List::$variant(elems) = $self.0 {
            elems.push($value);
            Ok(())
        } else {
            Err(<$error>::custom("NBT lists must be homogenous"))
        }
    };
}

impl<'de, 'a> Visitor<'de> for DeserializeListElement<'a> {
    type Value = ();

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "a valid NBT list element")
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: Error,
    {
        visit!(self, Byte, v, E)
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: Error,
    {
        visit!(self, Short, v, E)
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        visit!(self, Int, v, E)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        visit!(self, Long, v, E)
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        visit!(self, Float, v, E)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        visit!(self, Double, v, E)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        visit!(self, String, v, E)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        visit!(self, String, v.to_owned(), E)
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        visit!(self, List, ListVisitor.visit_seq(seq)?, A::Error)
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        visit!(self, Compound, visit_map(map)?, A::Error)
    }
}

fn visit_map<'de, A>(mut map: A) -> Result<Compound, A::Error>
where
    A: MapAccess<'de>,
{
    let mut compound = Compound::new();

    while let Some((k, v)) = map.next_entry()? {
        compound.insert(k, v);
    }

    Ok(compound)
}
