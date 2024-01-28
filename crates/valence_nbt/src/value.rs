use std::borrow::Cow;
use std::hash::Hash;

use crate::tag::Tag;
use crate::{Compound, List};

/// Represents an arbitrary NBT value.
#[derive(Clone, Debug)]
pub enum Value<S = String> {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(S),
    List(List<S>),
    Compound(Compound<S>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

/// Represents a reference to an arbitrary NBT value, where the tag is not part
/// of the reference.
#[derive(Copy, Clone, Debug)]
pub enum ValueRef<'a, S = String> {
    Byte(&'a i8),
    Short(&'a i16),
    Int(&'a i32),
    Long(&'a i64),
    Float(&'a f32),
    Double(&'a f64),
    ByteArray(&'a [i8]),
    String(&'a S),
    List(&'a List<S>),
    Compound(&'a Compound<S>),
    IntArray(&'a [i32]),
    LongArray(&'a [i64]),
}

/// Represents a mutable reference to an arbitrary NBT value, where the tag is
/// not part of the reference.
#[derive(Debug)]
pub enum ValueMut<'a, S = String> {
    Byte(&'a mut i8),
    Short(&'a mut i16),
    Int(&'a mut i32),
    Long(&'a mut i64),
    Float(&'a mut f32),
    Double(&'a mut f64),
    ByteArray(&'a mut Vec<i8>),
    String(&'a mut S),
    List(&'a mut List<S>),
    Compound(&'a mut Compound<S>),
    IntArray(&'a mut Vec<i32>),
    LongArray(&'a mut Vec<i64>),
}

macro_rules! impl_value {
    ($name:ident, $($lifetime:lifetime)?, ($($deref:tt)*), $($reference:tt)*) => {
        macro_rules! as_number {
            ($method_name:ident, $ty:ty, $($deref)*) => {
                #[doc = concat!("If this value is a number, returns the `", stringify!($ty), "` representation of this value.")]
                pub fn $method_name(&self) -> Option<$ty> {
                    #[allow(trivial_numeric_casts)]
                    match self {
                        Self::Byte(v) => Some($($deref)* v as $ty),
                        Self::Short(v) => Some($($deref)* v as $ty),
                        Self::Int(v) => Some($($deref)* v as $ty),
                        Self::Long(v) => Some($($deref)* v as $ty),
                        Self::Float(v) => Some(v.floor() as $ty),
                        Self::Double(v) => Some(v.floor() as $ty),
                        _ => None,
                    }
                }
            }
        }

        macro_rules! as_number_float {
            ($method_name:ident, $ty:ty, $($deref)*) => {
                #[doc = concat!("If this value is a number, returns the `", stringify!($ty), "` representation of this value.")]
                pub fn $method_name(&self) -> Option<$ty> {
                    #[allow(trivial_numeric_casts)]
                    match self {
                        Self::Byte(v) => Some($($deref)* v as $ty),
                        Self::Short(v) => Some($($deref)* v as $ty),
                        Self::Int(v) => Some($($deref)* v as $ty),
                        Self::Long(v) => Some($($deref)* v as $ty),
                        Self::Float(v) => Some($($deref)* v as $ty),
                        Self::Double(v) => Some($($deref)* v as $ty),
                        _ => None,
                    }
                }
            }
        }

        impl <$($lifetime,)? S> $name<$($lifetime,)? S> {
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

            /// Returns whether this value is a number, i.e. a byte, short, int, long, float or double.
            pub fn is_number(&self) -> bool {
                match self {
                    Self::Byte(_) | Self::Short(_) | Self::Int(_) | Self::Long(_) | Self::Float(_) | Self::Double(_) => true,
                    _ => false,
                }
            }

            /// Returns whether this value is a array, i.e. a byte array, int array or long array.
            pub fn is_array(&self) -> bool {
                match self {
                    Self::ByteArray(_) | Self::IntArray(_) | Self::LongArray(_) => true,
                    _ => false,
                }
            }

            /// Returns whether this value is a string.
            pub fn is_string(&self) -> bool {
                match self {
                    Self::String(_) => true,
                    _ => false,
                }
            }

            /// Returns whether this value is a list of values, i.e. a compound list, a string list and so on.
            pub fn is_list(&self) -> bool {
                match self {
                    Self::List(_) => true,
                    _ => false,
                }
            }

            /// Returns whether this value is a compound.
            pub fn is_compound(&self) -> bool {
                match self {
                    Self::Compound(_) => true,
                    _ => false,
                }
            }

            as_number!(as_i8, i8, $($deref)*);
            as_number!(as_i16, i16, $($deref)*);
            as_number!(as_i32, i32, $($deref)*);
            as_number!(as_i64, i64, $($deref)*);
            as_number_float!(as_f32, f32, $($deref)*);
            as_number_float!(as_f64, f64, $($deref)*);

            /// Returns the `String` representation of this value if it exists.
            pub fn as_string(&$($lifetime)* self) -> Option<&$($lifetime)* S> {
                match self {
                    Self::String(v) => Some(v),
                    _ => None,
                }
            }

            /// Returns the `List` representation of this value if it exists.
            pub fn as_list(&$($lifetime)* self) -> Option<&$($lifetime)* List<S>> {
                match self {
                    Self::List(v) => Some(v),
                    _ => None,
                }
            }

            /// Returns the `Compound` representation of this value if it exists.
            pub fn as_compound(&$($lifetime)* self) -> Option<&$($lifetime)* Compound<S>> {
                match self {
                    Self::Compound(v) => Some(v),
                    _ => None,
                }
            }

            /// If this value is a number, returns the `bool` representation of this value.
            pub fn as_bool(&self) -> Option<bool> {
                self.as_i8().map(|v| v != 0)
            }
        }

        impl <$($lifetime,)? S> From<$($reference)* i8> for $name<$($lifetime,)? S> {
            fn from(v: $($reference)* i8) -> Self {
                Self::Byte(v)
            }
        }

        impl <$($lifetime,)? S> From<$($reference)* i16> for $name<$($lifetime,)? S> {
            fn from(v: $($reference)* i16) -> Self {
                Self::Short(v)
            }
        }

        impl <$($lifetime,)? S> From<$($reference)* i32> for $name<$($lifetime,)? S> {
            fn from(v: $($reference)* i32) -> Self {
                Self::Int(v)
            }
        }

        impl <$($lifetime,)? S> From<$($reference)* i64> for $name<$($lifetime,)? S> {
            fn from(v: $($reference)* i64) -> Self {
                Self::Long(v)
            }
        }

        impl <$($lifetime,)? S> From<$($reference)* f32> for $name<$($lifetime,)? S> {
            fn from(v: $($reference)* f32) -> Self {
                Self::Float(v)
            }
        }

        impl <$($lifetime,)? S> From<$($reference)* f64> for $name<$($lifetime,)? S> {
            fn from(v: $($reference)* f64) -> Self {
                Self::Double(v)
            }
        }

        impl <$($lifetime,)? S> From<$($reference)* List<S>> for $name<$($lifetime,)? S> {
            fn from(v: $($reference)* List<S>) -> Self {
                Self::List(v)
            }
        }

        impl <$($lifetime,)? S> From<$($reference)* Compound<S>> for $name<$($lifetime,)? S> {
            fn from(v: $($reference)* Compound<S>) -> Self {
                Self::Compound(v)
            }
        }

        impl <$($lifetime,)? S> PartialEq<Self> for $name<$($lifetime,)? S> where S: Ord + Hash {
            fn eq(&self, other: &Self) -> bool {
                match self {
                    Self::Byte(v) => matches!(other, Self::Byte(other_v) if v == other_v),
                    Self::Short(v) => matches!(other, Self::Short(other_v) if v == other_v),
                    Self::Int(v) => matches!(other, Self::Int(other_v) if v == other_v),
                    Self::Long(v) => matches!(other, Self::Long(other_v) if v == other_v),
                    Self::Float(v) => matches!(other, Self::Float(other_v) if v == other_v),
                    Self::Double(v) => matches!(other, Self::Double(other_v) if v == other_v),
                    Self::ByteArray(v) => matches!(other, Self::ByteArray(other_v) if v == other_v),
                    Self::String(v) => matches!(other, Self::String(other_v) if v == other_v),
                    Self::List(v) => matches!(other, Self::List(other_v) if v == other_v),
                    Self::Compound(v) => matches!(other, Self::Compound(other_v) if v == other_v),
                    Self::IntArray(v) => matches!(other, Self::IntArray(other_v) if v == other_v),
                    Self::LongArray(v) => matches!(other, Self::LongArray(other_v) if v == other_v),
                }
            }
        }
    }
}

impl_value!(Value,,(*),);
impl_value!(ValueRef, 'a, (**), &'a);
impl_value!(ValueMut, 'a, (**), &'a mut);

impl<S> Value<S> {
    /// Returns the `[i8]` representation of this value if it exists.
    pub fn as_i8_array(&self) -> Option<&[i8]> {
        match self {
            Self::ByteArray(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the `[i32]` representation of this value if it exists.
    pub fn as_i32_array(&self) -> Option<&[i32]> {
        match self {
            Self::IntArray(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the `[i64]` representation of this value if it exists.
    pub fn as_i64_array(&self) -> Option<&[i64]> {
        match self {
            Self::LongArray(v) => Some(v),
            _ => None,
        }
    }

    /// Converts a reference to a value to a [`ValueRef`].
    pub fn as_value_ref(&self) -> ValueRef<S> {
        match self {
            Value::Byte(v) => ValueRef::Byte(v),
            Value::Short(v) => ValueRef::Short(v),
            Value::Int(v) => ValueRef::Int(v),
            Value::Long(v) => ValueRef::Long(v),
            Value::Float(v) => ValueRef::Float(v),
            Value::Double(v) => ValueRef::Double(v),
            Value::ByteArray(v) => ValueRef::ByteArray(&v[..]),
            Value::String(v) => ValueRef::String(v),
            Value::List(v) => ValueRef::List(v),
            Value::Compound(v) => ValueRef::Compound(v),
            Value::IntArray(v) => ValueRef::IntArray(&v[..]),
            Value::LongArray(v) => ValueRef::LongArray(&v[..]),
        }
    }

    /// Converts a mutable reference to a value to a [`ValueMut`].
    pub fn as_value_mut(&mut self) -> ValueMut<S> {
        match self {
            Value::Byte(v) => ValueMut::Byte(v),
            Value::Short(v) => ValueMut::Short(v),
            Value::Int(v) => ValueMut::Int(v),
            Value::Long(v) => ValueMut::Long(v),
            Value::Float(v) => ValueMut::Float(v),
            Value::Double(v) => ValueMut::Double(v),
            Value::ByteArray(v) => ValueMut::ByteArray(v),
            Value::String(v) => ValueMut::String(v),
            Value::List(v) => ValueMut::List(v),
            Value::Compound(v) => ValueMut::Compound(v),
            Value::IntArray(v) => ValueMut::IntArray(v),
            Value::LongArray(v) => ValueMut::LongArray(v),
        }
    }
}

impl<S> ValueRef<'_, S>
where
    S: Clone,
{
    /// Returns the `[i8]` representation of this value if it exists.
    pub fn as_i8_array(&self) -> Option<&[i8]> {
        match self {
            Self::ByteArray(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the `[i32]` representation of this value if it exists.
    pub fn as_i32_array(&self) -> Option<&[i32]> {
        match self {
            Self::IntArray(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the `[i64]` representation of this value if it exists.
    pub fn as_i64_array(&self) -> Option<&[i64]> {
        match self {
            Self::LongArray(v) => Some(v),
            _ => None,
        }
    }

    /// Clones this value reference to a new owned [`Value`].
    pub fn to_value(&self) -> Value<S> {
        match *self {
            ValueRef::Byte(v) => Value::Byte(*v),
            ValueRef::Short(v) => Value::Short(*v),
            ValueRef::Int(v) => Value::Int(*v),
            ValueRef::Long(v) => Value::Long(*v),
            ValueRef::Float(v) => Value::Float(*v),
            ValueRef::Double(v) => Value::Double(*v),
            ValueRef::ByteArray(v) => Value::ByteArray(v.to_vec()),
            ValueRef::String(v) => Value::String(v.to_owned()),
            ValueRef::List(v) => Value::List(v.clone()),
            ValueRef::Compound(v) => Value::Compound(v.clone()),
            ValueRef::IntArray(v) => Value::IntArray(v.to_vec()),
            ValueRef::LongArray(v) => Value::LongArray(v.to_vec()),
        }
    }
}

impl<S> ValueMut<'_, S>
where
    S: Clone,
{
    /// Returns the `[i8]` representation of this value if it exists.
    pub fn as_i8_array(&mut self) -> Option<&mut Vec<i8>> {
        match self {
            Self::ByteArray(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the `[i32]` representation of this value if it exists.
    pub fn as_i32_array(&mut self) -> Option<&mut Vec<i32>> {
        match self {
            Self::IntArray(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the `[i64]` representation of this value if it exists.
    pub fn as_i64_array(&mut self) -> Option<&mut Vec<i64>> {
        match self {
            Self::LongArray(v) => Some(v),
            _ => None,
        }
    }

    /// Clones this mutable value reference to a new owned [`Value`].
    pub fn to_value(&self) -> Value<S> {
        match self {
            ValueMut::Byte(v) => Value::Byte(**v),
            ValueMut::Short(v) => Value::Short(**v),
            ValueMut::Int(v) => Value::Int(**v),
            ValueMut::Long(v) => Value::Long(**v),
            ValueMut::Float(v) => Value::Float(**v),
            ValueMut::Double(v) => Value::Double(**v),
            ValueMut::ByteArray(v) => Value::ByteArray((*v).clone()),
            ValueMut::String(v) => Value::String((*v).clone()),
            ValueMut::List(v) => Value::List((*v).clone()),
            ValueMut::Compound(v) => Value::Compound((*v).clone()),
            ValueMut::IntArray(v) => Value::IntArray((*v).clone()),
            ValueMut::LongArray(v) => Value::LongArray((*v).clone()),
        }
    }
}

impl<'a, S> ValueMut<'a, S> {
    /// Downgrades this mutable value reference into an immutable [`ValueRef`].
    pub fn into_value_ref(self) -> ValueRef<'a, S> {
        match self {
            ValueMut::Byte(v) => ValueRef::Byte(v),
            ValueMut::Short(v) => ValueRef::Short(v),
            ValueMut::Int(v) => ValueRef::Int(v),
            ValueMut::Long(v) => ValueRef::Long(v),
            ValueMut::Float(v) => ValueRef::Float(v),
            ValueMut::Double(v) => ValueRef::Double(v),
            ValueMut::ByteArray(v) => ValueRef::ByteArray(&v[..]),
            ValueMut::String(v) => ValueRef::String(v),
            ValueMut::List(v) => ValueRef::List(v),
            ValueMut::Compound(v) => ValueRef::Compound(v),
            ValueMut::IntArray(v) => ValueRef::IntArray(&v[..]),
            ValueMut::LongArray(v) => ValueRef::LongArray(&v[..]),
        }
    }
}

/// Bools are usually represented as `0` or `1` bytes in NBT.
impl<S> From<bool> for Value<S> {
    fn from(b: bool) -> Self {
        Value::Byte(b as _)
    }
}

impl<S> From<Vec<i8>> for Value<S> {
    fn from(v: Vec<i8>) -> Self {
        Self::ByteArray(v)
    }
}

impl From<String> for Value<String> {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&String> for Value<String> {
    fn from(value: &String) -> Self {
        Self::String(value.clone())
    }
}

impl<'a> From<&'a str> for Value<String> {
    fn from(v: &'a str) -> Self {
        Self::String(v.to_owned())
    }
}

impl<'a> From<Cow<'a, str>> for Value<String> {
    fn from(v: Cow<'a, str>) -> Self {
        Self::String(v.into_owned())
    }
}

impl From<String> for Value<Cow<'_, str>> {
    fn from(v: String) -> Self {
        Self::String(Cow::Owned(v))
    }
}

impl<'a> From<&'a String> for Value<Cow<'a, str>> {
    fn from(v: &'a String) -> Self {
        Self::String(Cow::Borrowed(v))
    }
}

impl<'a> From<&'a str> for Value<Cow<'a, str>> {
    fn from(v: &'a str) -> Self {
        Self::String(Cow::Borrowed(v))
    }
}

impl<'a> From<Cow<'a, str>> for Value<Cow<'a, str>> {
    fn from(v: Cow<'a, str>) -> Self {
        Self::String(v)
    }
}

#[cfg(feature = "java_string")]
impl From<java_string::JavaString> for Value<java_string::JavaString> {
    fn from(v: java_string::JavaString) -> Self {
        Self::String(v)
    }
}

#[cfg(feature = "java_string")]
impl From<&java_string::JavaString> for Value<java_string::JavaString> {
    fn from(v: &java_string::JavaString) -> Self {
        Self::String(v.clone())
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<&'a java_string::JavaStr> for Value<java_string::JavaString> {
    fn from(v: &'a java_string::JavaStr) -> Self {
        Self::String(v.to_owned())
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<Cow<'a, java_string::JavaStr>> for Value<java_string::JavaString> {
    fn from(v: Cow<'a, java_string::JavaStr>) -> Self {
        Self::String(v.into_owned())
    }
}

#[cfg(feature = "java_string")]
impl From<String> for Value<java_string::JavaString> {
    fn from(v: String) -> Self {
        Self::String(java_string::JavaString::from(v))
    }
}

#[cfg(feature = "java_string")]
impl From<&String> for Value<java_string::JavaString> {
    fn from(v: &String) -> Self {
        Self::String(java_string::JavaString::from(v))
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<&'a str> for Value<java_string::JavaString> {
    fn from(v: &'a str) -> Self {
        Self::String(java_string::JavaString::from(v))
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<Cow<'a, str>> for Value<java_string::JavaString> {
    fn from(v: Cow<'a, str>) -> Self {
        Self::String(java_string::JavaString::from(v))
    }
}

#[cfg(feature = "java_string")]
impl From<java_string::JavaString> for Value<Cow<'_, java_string::JavaStr>> {
    fn from(v: java_string::JavaString) -> Self {
        Self::String(Cow::Owned(v))
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<&'a java_string::JavaString> for Value<Cow<'a, java_string::JavaStr>> {
    fn from(v: &'a java_string::JavaString) -> Self {
        Self::String(Cow::Borrowed(v))
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<&'a java_string::JavaStr> for Value<Cow<'a, java_string::JavaStr>> {
    fn from(v: &'a java_string::JavaStr) -> Self {
        Self::String(Cow::Borrowed(v))
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<Cow<'a, java_string::JavaStr>> for Value<Cow<'a, java_string::JavaStr>> {
    fn from(v: Cow<'a, java_string::JavaStr>) -> Self {
        Self::String(v)
    }
}

#[cfg(feature = "java_string")]
impl From<String> for Value<Cow<'_, java_string::JavaStr>> {
    fn from(v: String) -> Self {
        Self::String(Cow::Owned(java_string::JavaString::from(v)))
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<&'a String> for Value<Cow<'a, java_string::JavaStr>> {
    fn from(v: &'a String) -> Self {
        Self::String(Cow::Borrowed(java_string::JavaStr::from_str(v)))
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<&'a str> for Value<Cow<'a, java_string::JavaStr>> {
    fn from(v: &'a str) -> Self {
        Self::String(Cow::Borrowed(java_string::JavaStr::from_str(v)))
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<Cow<'a, str>> for Value<Cow<'a, java_string::JavaStr>> {
    fn from(v: Cow<'a, str>) -> Self {
        Self::String(match v {
            Cow::Borrowed(str) => Cow::Borrowed(java_string::JavaStr::from_str(str)),
            Cow::Owned(str) => Cow::Owned(java_string::JavaString::from(str)),
        })
    }
}

impl<S> From<Vec<i32>> for Value<S> {
    fn from(v: Vec<i32>) -> Self {
        Self::IntArray(v)
    }
}

impl<S> From<Vec<i64>> for Value<S> {
    fn from(v: Vec<i64>) -> Self {
        Self::LongArray(v)
    }
}

impl<S> From<ValueRef<'_, S>> for Value<S>
where
    S: Clone,
{
    fn from(v: ValueRef<S>) -> Self {
        v.to_value()
    }
}

impl<S> From<&ValueRef<'_, S>> for Value<S>
where
    S: Clone,
{
    fn from(v: &ValueRef<S>) -> Self {
        v.to_value()
    }
}

impl<S> From<ValueMut<'_, S>> for Value<S>
where
    S: Clone,
{
    fn from(v: ValueMut<S>) -> Self {
        v.to_value()
    }
}

impl<S> From<&ValueMut<'_, S>> for Value<S>
where
    S: Clone,
{
    fn from(v: &ValueMut<S>) -> Self {
        v.to_value()
    }
}

#[cfg(feature = "uuid")]
impl<S> From<uuid::Uuid> for Value<S> {
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
impl<I, S> From<valence_ident::Ident<I>> for Value<S>
where
    I: Into<Value<S>>,
{
    fn from(value: valence_ident::Ident<I>) -> Self {
        value.into_inner().into()
    }
}

impl<'a> From<&'a [i8]> for ValueRef<'a> {
    fn from(v: &'a [i8]) -> Self {
        Self::ByteArray(v)
    }
}

impl<'a> From<&'a String> for ValueRef<'a, String> {
    fn from(v: &'a String) -> ValueRef<'a> {
        Self::String(v)
    }
}

impl<'a, S> From<&'a [i32]> for ValueRef<'a, S> {
    fn from(v: &'a [i32]) -> Self {
        Self::IntArray(v)
    }
}

impl<'a, S> From<&'a [i64]> for ValueRef<'a, S> {
    fn from(v: &'a [i64]) -> Self {
        Self::LongArray(v)
    }
}

impl<'a, S> From<&'a Value<S>> for ValueRef<'a, S> {
    fn from(v: &'a Value<S>) -> Self {
        v.as_value_ref()
    }
}

impl<'a, S> From<ValueMut<'a, S>> for ValueRef<'a, S> {
    fn from(v: ValueMut<'a, S>) -> Self {
        v.into_value_ref()
    }
}

#[cfg(feature = "valence_ident")]
impl<'a> From<&'a valence_ident::Ident<String>> for ValueRef<'a, String> {
    fn from(v: &'a valence_ident::Ident<String>) -> Self {
        Self::String(v.as_ref())
    }
}

impl<'a, S> From<&'a mut Vec<i8>> for ValueMut<'a, S> {
    fn from(v: &'a mut Vec<i8>) -> Self {
        Self::ByteArray(v)
    }
}

impl<'a> From<&'a mut String> for ValueMut<'a, String> {
    fn from(v: &'a mut String) -> Self {
        Self::String(v)
    }
}

impl<'a, S> From<&'a mut Vec<i32>> for ValueMut<'a, S> {
    fn from(v: &'a mut Vec<i32>) -> Self {
        Self::IntArray(v)
    }
}

impl<'a, S> From<&'a mut Vec<i64>> for ValueMut<'a, S> {
    fn from(v: &'a mut Vec<i64>) -> Self {
        Self::LongArray(v)
    }
}

impl<'a, S> From<&'a mut Value<S>> for ValueMut<'a, S> {
    fn from(v: &'a mut Value<S>) -> Self {
        v.as_value_mut()
    }
}
