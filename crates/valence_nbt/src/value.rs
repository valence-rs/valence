use std::borrow::Cow;

use crate::tag::Tag;
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
    /// The list with the element type of `TAG_End` and length of zero.
    End,
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
            List::End => 0,
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

    /// Returns the element type of this list.
    pub fn element_tag(&self) -> Tag {
        match self {
            List::End => Tag::End,
            List::Byte(_) => Tag::Byte,
            List::Short(_) => Tag::Short,
            List::Int(_) => Tag::Int,
            List::Long(_) => Tag::Long,
            List::Float(_) => Tag::Float,
            List::Double(_) => Tag::Double,
            List::ByteArray(_) => Tag::ByteArray,
            List::String(_) => Tag::String,
            List::List(_) => Tag::List,
            List::Compound(_) => Tag::Compound,
            List::IntArray(_) => Tag::IntArray,
            List::LongArray(_) => Tag::LongArray,
        }
    }
}

/// We can not create new identities in stable Rust using macros, so we provide
/// them in the macro invocation itself.
macro_rules! nbt_conversion {
    ( $($nbt_type:ident = $value_type:ty => $is_function:ident $as_function:ident $as_mut_function:ident $into_function:ident)+ ) => {
        $(
            pub fn $is_function(&self) -> bool {
                self.$as_function().is_some()
            }

            pub fn $as_function(&self) -> Option<&$value_type> {
                match self {
                    Self::$nbt_type(value) => Some(value),
                    _ => None
                }
            }

            pub fn $as_mut_function(&mut self) -> Option<&mut $value_type> {
                match self {
                    Self::$nbt_type(value) => Some(value),
                    _ => None
                }
            }

            pub fn $into_function(self) -> Option<$value_type> {
                match self {
                    Self::$nbt_type(value) => Some(value),
                    _ => None
                }
            }
        )*
    };
}

impl Value {
    nbt_conversion! {
        Byte = i8 => is_byte as_byte as_byte_mut into_byte
        Short = i16 => is_short as_short as_short_mut into_short
        Int = i32 => is_int as_int as_int_mut into_int
        Long = i64 => is_long as_long as_long_mut into_long
        Float = f32 => is_float as_float as_float_mut into_float
        Double = f64 => is_double as_double as_double_mut into_double
        ByteArray = Vec<i8> => is_byte_array as_byte_array as_byte_array_mut into_byte_array
        String = String => is_string as_string as_string_mut into_string
        List = List => is_list as_list as_list_mut into_list
        Compound = Compound => is_compound as_compound as_compound_mut into_compound
        IntArray = Vec<i32> => is_int_array as_int_array as_int_array_mut into_int_array
        LongArray = Vec<i64> => is_long_array as_long_array as_long_array_mut into_long_array
    }

    /// Returns the type of this value.
    pub fn get_tag(&self) -> Tag {
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
