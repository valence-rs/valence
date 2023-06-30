use std::fmt;
use std::mem::ManuallyDrop;

pub use ser::*;
use thiserror::Error;

mod de;
mod ser;
#[cfg(test)]
mod tests;

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
