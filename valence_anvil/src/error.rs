use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::io;

use valence::ident::Ident;

/// Errors that can occur when encoding or decoding.
#[derive(Debug)]
pub struct Error {
    /// Box this to keep the size of `Result<T, Error>` small.
    cause: Box<Cause>,
}

impl Error {
    pub(crate) fn unknown_compression_scheme(mode: u8) -> Self {
        Self {
            cause: Box::new(Cause::Parse(ParseError::UnknownCompressionScheme(mode))),
        }
    }

    pub(crate) fn invalid_chunk_size(size: usize) -> Self {
        Self {
            cause: Box::new(Cause::Parse(ParseError::InvalidChunkSize(size))),
        }
    }

    pub(crate) fn missing_nbt_value(key: &'static str) -> Self {
        Self {
            cause: Box::new(Cause::Parse(ParseError::MissingNBT(key))),
        }
    }

    pub(crate) fn invalid_nbt(message: &'static str) -> Self {
        Self {
            cause: Box::new(Cause::Parse(ParseError::InvalidNBT(message))),
        }
    }

    pub(crate) fn invalid_palette() -> Self {
        Self {
            cause: Box::new(Cause::Parse(ParseError::InvalidPalette)),
        }
    }

    pub(crate) fn unknown_type(ident: Ident<String>) -> Self {
        Self {
            cause: Box::new(Cause::Parse(ParseError::UnknownType(ident))),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &*self.cause {
            Cause::Io(e) => Some(e),
            _ => None,
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
impl From<valence::nbt::Error> for Error {
    fn from(e: valence::nbt::Error) -> Self {
        Self {
            cause: Box::new(Cause::NBT(e)),
        }
    }
}

impl From<valence::ident::IdentError<String>> for Error {
    fn from(e: valence::ident::IdentError<String>) -> Self {
        Self {
            cause: Box::new(Cause::IdentityError(e)),
        }
    }
}

#[derive(Debug)]
pub enum Cause {
    Io(io::Error),
    Parse(ParseError),
    NBT(valence::nbt::Error),
    IdentityError(valence::ident::IdentError<String>),
}

#[derive(Debug)]
pub enum ParseError {
    UnknownCompressionScheme(u8),
    InvalidChunkSize(usize),
    MissingNBT(&'static str),
    InvalidNBT(&'static str),
    InvalidPalette,
    UnknownType(Ident<String>),
}

#[derive(Debug)]
pub enum SerializeError {
    //    ChunkTooLarge
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &*self.cause {
            Cause::Io(e) => e.fmt(f),
            Cause::Parse(err) => err.fmt(f),
            Cause::NBT(e) => e.fmt(f),
            Cause::IdentityError(e) => e.fmt(f),
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> valence::vek::serde::__private::fmt::Result {
        write!(f, "Parse failed")
    }
}

impl Display for SerializeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> valence::vek::serde::__private::fmt::Result {
        write!(f, "Serialization failed")
    }
}
