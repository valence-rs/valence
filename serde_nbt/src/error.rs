use std::error::Error as StdError;
use std::fmt::Display;
use std::iter::FusedIterator;
use std::{fmt, io};

use serde::{de, ser};

/// Errors that can occur when serializing or deserializing.
///
/// The error type maintains a backtrace through the NBT value which caused the
/// error. This is used in the `Display` impl on the error.
#[derive(Debug)]
pub struct Error {
    /// Box this to keep the error as small as possible. We don't want to
    /// slow down the common case where no error occurs.
    inner: Box<ErrorInner>,
}

#[derive(Debug)]
struct ErrorInner {
    trace: Vec<String>,
    cause: Cause,
}

#[derive(Debug)]
enum Cause {
    Io(io::Error),
    // catch-all errors
    Owned(Box<str>),
    Static(&'static str),
}

impl Error {
    pub(crate) fn new_owned(msg: impl Into<Box<str>>) -> Self {
        Self {
            inner: Box::new(ErrorInner {
                trace: Vec::new(),
                cause: Cause::Owned(msg.into()),
            }),
        }
    }

    pub(crate) fn new_static(msg: &'static str) -> Self {
        Self {
            inner: Box::new(ErrorInner {
                trace: Vec::new(),
                cause: Cause::Static(msg),
            }),
        }
    }

    pub(crate) fn field(mut self, ctx: impl Into<String>) -> Self {
        self.inner.trace.push(ctx.into());
        self
    }

    /// Returns an iterator through the nested fields of an NBT compound to the
    /// location where the error occurred.
    ///
    /// The iterator's `Item` is the name of the current field.
    pub fn trace(
        &self,
    ) -> impl DoubleEndedIterator<Item = &str> + ExactSizeIterator + FusedIterator + Clone + '_
    {
        self.inner.trace.iter().rev().map(|s| s.as_str())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let len = self.inner.trace.len();

        if len > 0 {
            write!(f, "(")?;
            for (i, ctx) in self.trace().enumerate() {
                write!(f, "{ctx}")?;

                if i != len - 1 {
                    write!(f, " → ")?;
                }
            }
            write!(f, ") ")?;
        }

        match &self.inner.cause {
            Cause::Io(e) => e.fmt(f),
            Cause::Owned(s) => write!(f, "{s}"),
            Cause::Static(s) => write!(f, "{s}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &self.inner.cause {
            Cause::Io(e) => Some(e),
            Cause::Owned(_) => None,
            Cause::Static(_) => None,
        }
    }
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::new_owned(format!("{msg}"))
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error::new_owned(format!("{msg}"))
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self {
            inner: Box::new(ErrorInner {
                trace: Vec::new(),
                cause: Cause::Io(e),
            }),
        }
    }
}
