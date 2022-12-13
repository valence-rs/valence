use std::fmt;
use std::fmt::Formatter;

use crate::{Compound, List, Value};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tag {
    // Variant order is significant!
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
    pub fn element_type(value: &Value) -> Self {
        match value {
            Value::Byte(_) => Tag::Byte,
            Value::Short(_) => Tag::Short,
            Value::Int(_) => Tag::Int,
            Value::Long(_) => Tag::Long,
            Value::Float(_) => Tag::Float,
            Value::Double(_) => Tag::Double,
            Value::ByteArray(_) => Tag::ByteArray,
            Value::String(_) => Tag::String,
            Value::List(_) => Tag::List,
            Value::Compound(_) => Tag::Compound,
            Value::IntArray(_) => Tag::IntArray,
            Value::LongArray(_) => Tag::LongArray,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
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
        }
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

pub trait NbtType {
    const TAG: Tag;
}

impl NbtType for i8 {
    const TAG: Tag = Tag::Byte;
}

impl NbtType for i16 {
    const TAG: Tag = Tag::Short;
}

impl NbtType for i32 {
    const TAG: Tag = Tag::Int;
}

impl NbtType for i64 {
    const TAG: Tag = Tag::Long;
}

impl NbtType for f32 {
    const TAG: Tag = Tag::Float;
}

impl NbtType for f64 {
    const TAG: Tag = Tag::Double;
}

impl NbtType for Vec<i8> {
    const TAG: Tag = Tag::ByteArray;
}

impl NbtType for String {
    const TAG: Tag = Tag::String;
}

impl NbtType for List {
    const TAG: Tag = Tag::List;
}

impl NbtType for Compound {
    const TAG: Tag = Tag::Compound;
}

impl NbtType for Vec<i32> {
    const TAG: Tag = Tag::IntArray;
}

impl NbtType for Vec<i64> {
    const TAG: Tag = Tag::LongArray;
}
