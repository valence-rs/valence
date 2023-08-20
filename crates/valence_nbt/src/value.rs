use std::borrow::Cow;

use crate::tag::Tag;
use crate::Compound;
// need pub use here for backwards compatibility (List used to be defined here)
pub use crate::List;

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

/// Represents a reference to an arbitrary NBT value, where the tag is not part
/// of the reference.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ValueRef<'a> {
    Byte(&'a i8),
    Short(&'a i16),
    Int(&'a i32),
    Long(&'a i64),
    Float(&'a f32),
    Double(&'a f64),
    ByteArray(&'a [i8]),
    String(&'a str),
    List(&'a List),
    Compound(&'a Compound),
    IntArray(&'a [i32]),
    LongArray(&'a [i64]),
}

/// Represents a mutable reference to an arbitrary NBT value, where the tag is
/// not part of the reference.
#[derive(PartialEq, Debug)]
pub enum ValueRefMut<'a> {
    Byte(&'a mut i8),
    Short(&'a mut i16),
    Int(&'a mut i32),
    Long(&'a mut i64),
    Float(&'a mut f32),
    Double(&'a mut f64),
    ByteArray(&'a mut Vec<i8>),
    String(&'a mut String),
    List(&'a mut List),
    Compound(&'a mut Compound),
    IntArray(&'a mut Vec<i32>),
    LongArray(&'a mut Vec<i64>),
}

macro_rules! impl_value {
    ($name:ident, $($lifetime:lifetime)?, $($reference:tt)*) => {
        impl $(<$lifetime>)? $name $(<$lifetime>)? {
            /// Returns the type of this value.
            pub fn tag(&self) -> Tag {
                match self {
                    Self::Byte(_) => Tag::Byte,
                    Self::Short(_) => Tag::Short,
                    Self::Int(_) => Tag::Int,
                    Self::Long(_) => Tag::Long,
                    Self::Float(_) => Tag::Float,
                    Self::Double(_) => Tag::Double,
                    Self::ByteArray(_) => Tag::ByteArray,
                    Self::String(_) => Tag::String,
                    Self::List(_) => Tag::List,
                    Self::Compound(_) => Tag::Compound,
                    Self::IntArray(_) => Tag::IntArray,
                    Self::LongArray(_) => Tag::LongArray,
                }
            }
        }

        impl $(<$lifetime>)? From<$($reference)* i8> for $name $(<$lifetime>)? {
            fn from(v: $($reference)* i8) -> Self {
                Self::Byte(v)
            }
        }

        impl $(<$lifetime>)? From<$($reference)* i16> for $name $(<$lifetime>)? {
            fn from(v: $($reference)* i16) -> Self {
                Self::Short(v)
            }
        }

        impl $(<$lifetime>)? From<$($reference)* i32> for $name $(<$lifetime>)? {
            fn from(v: $($reference)* i32) -> Self {
                Self::Int(v)
            }
        }

        impl $(<$lifetime>)? From<$($reference)* i64> for $name $(<$lifetime>)? {
            fn from(v: $($reference)* i64) -> Self {
                Self::Long(v)
            }
        }

        impl $(<$lifetime>)? From<$($reference)* f32> for $name $(<$lifetime>)? {
            fn from(v: $($reference)* f32) -> Self {
                Self::Float(v)
            }
        }

        impl $(<$lifetime>)? From<$($reference)* f64> for $name $(<$lifetime>)? {
            fn from(v: $($reference)* f64) -> Self {
                Self::Double(v)
            }
        }

        impl $(<$lifetime>)? From<$($reference)* List> for $name $(<$lifetime>)? {
            fn from(v: $($reference)* List) -> Self {
                Self::List(v)
            }
        }

        impl $(<$lifetime>)? From<$($reference)* Compound> for $name $(<$lifetime>)? {
            fn from(v: $($reference)* Compound) -> Self {
                Self::Compound(v)
            }
        }
    }
}

impl_value!(Value,,);
impl_value!(ValueRef, 'a, &'a);
impl_value!(ValueRefMut, 'a, &'a mut);

/// Bools are usually represented as `0` or `1` bytes in NBT.
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Byte(b as _)
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

impl From<&String> for Value {
    fn from(value: &String) -> Self {
        Self::String(value.clone())
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

#[cfg(feature = "uuid")]
impl From<uuid::Uuid> for Value {
    fn from(value: uuid::Uuid) -> Self {
        let (most, least) = value.as_u64_pair();

        let first = (most >> 32) as i32;
        let second = most as i32;
        let third = (least >> 32) as i32;
        let fourth = least as i32;

        Value::IntArray(vec![first, second, third, fourth])
    }
}

#[cfg(feature = "valence_ident")]
impl<S> From<valence_ident::Ident<S>> for Value
where
    S: Into<Value>,
{
    fn from(value: valence_ident::Ident<S>) -> Self {
        value.into_inner().into()
    }
}

impl<'a> From<&'a [i8]> for ValueRef<'a> {
    fn from(v: &'a [i8]) -> Self {
        Self::ByteArray(v)
    }
}

impl<'a> From<&'a str> for ValueRef<'a> {
    fn from(v: &'a str) -> ValueRef<'a> {
        Self::String(v)
    }
}

impl<'a> From<&'a Cow<'_, str>> for ValueRef<'a> {
    fn from(v: &'a Cow<'_, str>) -> Self {
        Self::String(v.as_ref())
    }
}

impl<'a> From<&'a [i32]> for ValueRef<'a> {
    fn from(v: &'a [i32]) -> Self {
        Self::IntArray(v)
    }
}

impl<'a> From<&'a [i64]> for ValueRef<'a> {
    fn from(v: &'a [i64]) -> Self {
        Self::LongArray(v)
    }
}

#[cfg(feature = "valence_ident")]
impl<'a, S> From<&'a valence_ident::Ident<S>> for ValueRef<'a>
where
    S: AsRef<str>,
{
    fn from(v: &'a valence_ident::Ident<S>) -> Self {
        Self::String(v.as_ref())
    }
}

impl<'a> From<&'a mut Vec<i8>> for ValueRefMut<'a> {
    fn from(v: &'a mut Vec<i8>) -> Self {
        Self::ByteArray(v)
    }
}

impl<'a> From<&'a mut String> for ValueRefMut<'a> {
    fn from(v: &'a mut String) -> Self {
        Self::String(v)
    }
}

impl<'a> From<&'a mut Vec<i32>> for ValueRefMut<'a> {
    fn from(v: &'a mut Vec<i32>) -> Self {
        Self::IntArray(v)
    }
}

impl<'a> From<&'a mut Vec<i64>> for ValueRefMut<'a> {
    fn from(v: &'a mut Vec<i64>) -> Self {
        Self::LongArray(v)
    }
}
