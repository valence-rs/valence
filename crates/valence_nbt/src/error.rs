use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::io;

pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors that can occur when encoding or decoding binary NBT.
#[derive(Debug)]
pub struct Error {
    /// Box this to keep the size of `Result<T, Error>` small.
    cause: Box<Cause>,
}

#[derive(Debug)]
enum Cause {
    Io(io::Error),
    Owned(Box<str>),
    Static(&'static str),
}

impl Error {
    pub(crate) fn new_owned(msg: impl Into<Box<str>>) -> Self {
        Self {
            cause: Box::new(Cause::Owned(msg.into())),
        }
    }

    pub(crate) fn new_static(msg: &'static str) -> Self {
        Self {
            cause: Box::new(Cause::Static(msg)),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &*self.cause {
            Cause::Io(e) => e.fmt(f),
            Cause::Owned(msg) => write!(f, "{msg}"),
            Cause::Static(msg) => write!(f, "{msg}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &*self.cause {
            Cause::Io(e) => Some(e),
            Cause::Owned(_) => None,
            Cause::Static(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self {
            cause: Box::new(Cause::Io(e)),
        }
    }
}
