//! A [serde] library for the serialization and deserialization of Minecraft's
//! [Named Binary Tag] (NBT) format.
//!
//! [serde]: https://docs.rs/serde/latest/serde/
//! [Named Binary Tag]: https://minecraft.fandom.com/wiki/NBT_format

use std::fmt;
use std::fmt::{Display, Formatter};

pub use array::*;
pub use error::*;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
pub use value::*;

mod array;
mod error;
mod value;

#[cfg(test)]
mod tests;

/// (De)serialization support for the binary representation of NBT.
pub mod binary {
    pub use de::*;
    pub use ser::*;

    mod de;
    mod ser;
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Tag {
    End,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    ByteArray,
    String,
    List,
    Compound,
    IntArray,
    LongArray,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ArrayType {
    Byte,
    Int,
    Long,
}

impl ArrayType {
    pub const fn element_tag(self) -> Tag {
        match self {
            ArrayType::Byte => Tag::Byte,
            ArrayType::Int => Tag::Int,
            ArrayType::Long => Tag::Long,
        }
    }
}

impl<'de> Deserialize<'de> for ArrayType {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ArrayTypeVisitor;

        impl<'de> Visitor<'de> for ArrayTypeVisitor {
            type Value = ArrayType;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                write!(formatter, "a u8 or string encoding an NBT array type")
            }

            fn visit_u8<E>(self, v: u8) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    0 => Ok(ArrayType::Byte),
                    1 => Ok(ArrayType::Int),
                    2 => Ok(ArrayType::Long),
                    i => Err(E::custom(format!("invalid array type index `{i}`"))),
                }
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    BYTE_ARRAY_VARIANT_NAME => Ok(ArrayType::Byte),
                    INT_ARRAY_VARIANT_NAME => Ok(ArrayType::Int),
                    LONG_ARRAY_VARIANT_NAME => Ok(ArrayType::Long),
                    s => Err(E::custom(format!("invalid array type `{s}`"))),
                }
            }
        }

        deserializer.deserialize_u8(ArrayTypeVisitor)
    }
}

impl Tag {
    pub fn from_u8(id: u8) -> Result<Self> {
        match id {
            0 => Ok(Tag::End),
            1 => Ok(Tag::Byte),
            2 => Ok(Tag::Short),
            3 => Ok(Tag::Int),
            4 => Ok(Tag::Long),
            5 => Ok(Tag::Float),
            6 => Ok(Tag::Double),
            7 => Ok(Tag::ByteArray),
            8 => Ok(Tag::String),
            9 => Ok(Tag::List),
            10 => Ok(Tag::Compound),
            11 => Ok(Tag::IntArray),
            12 => Ok(Tag::LongArray),
            _ => Err(Error::new_owned(format!("invalid tag byte `{id}`"))),
        }
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Tag::End => "end",
            Tag::Byte => "byte",
            Tag::Short => "short",
            Tag::Int => "int",
            Tag::Long => "long",
            Tag::Float => "float",
            Tag::Double => "double",
            Tag::ByteArray => "byte array",
            Tag::String => "string",
            Tag::List => "list",
            Tag::Compound => "compound",
            Tag::IntArray => "int array",
            Tag::LongArray => "long array",
        };

        write!(f, "{name}")
    }
}

/// Error message for cesu-8 decoding failures.
const CESU8_DECODE_ERROR: &str = "could not convert CESU-8 data to UTF-8";

/// The name of the enum used to encode arrays.
const ARRAY_ENUM_NAME: &str = "__array__";

const BYTE_ARRAY_VARIANT_NAME: &str = "__byte_array__";
const INT_ARRAY_VARIANT_NAME: &str = "__int_array__";
const LONG_ARRAY_VARIANT_NAME: &str = "__long_array__";
