use std::io;

use thiserror::Error;
use valence::ident::{Ident, IdentError};

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    DataFormatError(#[from] DataFormatError),
    #[error(transparent)]
    NbtParseError(#[from] valence::nbt::Error),
    #[error(transparent)]
    NbtFormatError(#[from] NbtFormatError),
}

#[derive(Error, Debug)]
pub enum NbtFormatError {
    #[error("Missing key: {0}")]
    MissingKey(String),
    #[error("Invalid type: {0}")]
    InvalidType(String),
}

#[derive(Error, Debug)]
pub enum DataFormatError {
    #[error("Unknown compression scheme: {0}")]
    UnknownCompressionScheme(u8),
    #[error("Invalid chunk size: {0}")]
    InvalidChunkSize(usize),
    #[error(transparent)]
    IdentityError(#[from] IdentError<String>),
    #[error("Unknown identity: {0}")]
    UnknownType(Ident<String>),
    #[error("Invalid chunk state: {0}")]
    InvalidChunkState(String),
    #[error("Invalid chunk palette")]
    InvalidPalette,
}

impl From<IdentError<String>> for Error {
    fn from(err: IdentError<String>) -> Self {
        Self::DataFormatError(DataFormatError::IdentityError(err))
    }
}
