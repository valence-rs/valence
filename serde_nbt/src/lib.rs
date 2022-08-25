use std::fmt;
use std::fmt::{Display, Formatter};
use anyhow::anyhow;

pub use error::*;
pub use value::*;

mod error;
mod value;
mod array;

#[cfg(test)]
mod tests;

pub use array::*;

/// (De)serialization support for the binary representation of NBT.
pub mod binary {
    pub use ser::*;
    pub use de::*;

    mod ser;
    mod de;
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
            _ => Err(Error(anyhow!("invalid tag byte `{id}`")))
        }
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
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

        write!(f, "{s}")
    }
}

const BYTE_ARRAY_MAGIC: &str = "__byte_array__";
const INT_ARRAY_MAGIC: &str = "__int_array__";
const LONG_ARRAY_MAGIC: &str = "__long_array__";
