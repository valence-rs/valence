use std::borrow::Cow;

use crate::Compound;

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

/// An NBT list value.
///
/// NBT lists are homogeneous, meaning each list element must be of the same
/// type. This is opposed to a format like JSON where lists can be
/// heterogeneous. Here is a JSON list that would be illegal in NBT:
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
    /// Returns the length of this list.
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

    /// Returns `true` if this list has no elements. `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<i8> for Value {
    fn from(v: i8) -> Self {
        Self::Byte(v)
    }
}

/// Bools are usually represented as `0` or `1` bytes in NBT.
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Byte(b as _)
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

impl From<Value> for Option<i8> {
    fn from(value: Value) -> Self {
        if let Value::Byte(b) = value {
            Some(b)
        } else {
            None
        }
    }
}

impl From<Value> for Option<i16> {
    fn from(value: Value) -> Self {
        if let Value::Short(val) = value {
            Some(val)
        } else {
            None
        }
    }
}

impl From<Value> for Option<i32> {
    fn from(value: Value) -> Self {
        if let Value::Int(val) = value {
            Some(val)
        } else {
            None
        }
    }
}

impl From<Value> for Option<i64> {
    fn from(value: Value) -> Self {
        if let Value::Long(val) = value {
            Some(val)
        } else {
            None
        }
    }
}

impl From<Value> for Option<f32> {
    fn from(value: Value) -> Self {
        if let Value::Float(val) = value {
            Some(val)
        } else {
            None
        }
    }
}

impl From<Value> for Option<f64> {
    fn from(value: Value) -> Self {
        if let Value::Double(val) = value {
            Some(val)
        } else {
            None
        }
    }
}

impl From<Value> for Option<Vec<i8>> {
    fn from(value: Value) -> Self {
        if let Value::ByteArray(val) = value {
            Some(val)
        } else {
            None
        }
    }
}

impl From<Value> for Option<String> {
    fn from(value: Value) -> Self {
        if let Value::String(val) = value {
            Some(val)
        } else {
            None
        }
    }
}

impl From<Value> for Option<List> {
    fn from(value: Value) -> Self {
        if let Value::List(val) = value {
            Some(val)
        } else {
            None
        }
    }
}

impl From<Value> for Option<Compound> {
    fn from(value: Value) -> Self {
        if let Value::Compound(val) = value {
            Some(val)
        } else {
            None
        }
    }
}

impl From<Value> for Option<Vec<i32>> {
    fn from(value: Value) -> Self {
        if let Value::IntArray(val) = value {
            Some(val)
        } else {
            None
        }
    }
}

impl From<Value> for Option<Vec<i64>> {
    fn from(value: Value) -> Self {
        if let Value::LongArray(val) = value {
            Some(val)
        } else {
            None
        }
    }
}
