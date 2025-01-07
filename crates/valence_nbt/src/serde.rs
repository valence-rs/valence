#![allow(unused_imports)]
use std::fmt;

use ser::*;

use crate::Error;

pub mod de;
pub mod ser;
#[cfg(test)]
mod tests;

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::new_owned(format!("{msg}"))
    }
}

impl serde::ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        Self::new_owned(format!("{msg}"))
    }
}
