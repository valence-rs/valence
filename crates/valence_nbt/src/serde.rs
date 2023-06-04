use std::fmt;
use std::mem::ManuallyDrop;

pub use ser::*;
use thiserror::Error;

mod de;
mod ser;

/// Errors that can occur while serializing or deserializing.
#[derive(Clone, Error, Debug)]
#[error("{0}")]

pub struct Error(Box<str>);

impl Error {
    fn new(s: impl Into<Box<str>>) -> Self {
        Self(s.into())
    }
}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::new(format!("{msg}"))
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::new(format!("{msg}"))
    }
}

#[inline]
fn u8_vec_to_i8_vec(vec: Vec<u8>) -> Vec<i8> {
    // SAFETY: Layouts of u8 and i8 are the same and we're being careful not to drop
    // the original vec after calling Vec::from_raw_parts.
    unsafe {
        let mut vec = ManuallyDrop::new(vec);
        Vec::from_raw_parts(vec.as_mut_ptr() as *mut i8, vec.len(), vec.capacity())
    }
}

#[inline]
fn i8_vec_to_u8_vec(vec: Vec<i8>) -> Vec<u8> {
    // SAFETY: Layouts of u8 and i8 are the same and we're being careful not to drop
    // the original vec after calling Vec::from_raw_parts.
    unsafe {
        let mut vec = ManuallyDrop::new(vec);
        Vec::from_raw_parts(vec.as_mut_ptr() as *mut u8, vec.len(), vec.capacity())
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde::{Deserialize, Serialize};
    use serde_json::json;

    use super::*;
    use crate::{compound, Compound, List};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Struct {
        foo: i32,
        bar: StructInner,
        baz: String,
        quux: Vec<f32>,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct StructInner {
        a: bool,
        b: i64,
        c: Vec<Vec<i32>>,
        d: Vec<StructInner>,
    }

    fn make_struct() -> Struct {
        Struct {
            foo: i32::MIN,
            bar: StructInner {
                a: true,
                b: 123456789,
                c: vec![vec![1, 2, 3], vec![4, 5, 6]],
                d: vec![],
            },
            baz: "🤨".into(),
            quux: vec![std::f32::consts::PI, f32::MAX, f32::MIN],
        }
    }

    fn make_compound() -> Compound {
        compound! {
            "foo" => i32::MIN,
            "bar" => compound! {
                "a" => true,
                "b" => 123456789_i64,
                "c" => List::IntArray(vec![vec![1, 2, 3], vec![4, 5, 6]]),
                "d" => List::End,
            },
            "baz" => "🤨",
            "quux" => List::Float(vec![
                std::f32::consts::PI,
                f32::MAX,
                f32::MIN,
            ]),
        }
    }

    fn make_json() -> serde_json::Value {
        json!({
            "foo": i32::MIN,
            "bar": {
                "a": true,
                "b": 123456789_i64,
                "c": [[1, 2, 3], [4, 5, 6]],
                "d": []
            },
            "baz": "🤨",
            "quux": [
                std::f32::consts::PI,
                f32::MAX,
                f32::MIN,
            ]
        })
    }

    #[test]
    fn struct_to_compound() {
        let c = make_struct().serialize(CompoundSerializer).unwrap();

        assert_eq!(c, make_compound());
    }

    #[test]
    fn compound_to_struct() {
        let s = Struct::deserialize(make_compound()).unwrap();

        assert_eq!(s, make_struct());
    }

    #[test]
    fn compound_to_json() {
        let mut j = serde_json::to_value(make_compound()).unwrap();

        // Bools map to bytes in NBT, but the result should be the same otherwise.
        let p = j.pointer_mut("/bar/a").unwrap();
        assert_eq!(*p, serde_json::Value::from(1));
        *p = true.into();

        assert_eq!(j, make_json());
    }
}
