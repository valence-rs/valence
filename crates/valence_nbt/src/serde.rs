use std::fmt;

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
