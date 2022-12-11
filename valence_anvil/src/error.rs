use std::io;

use thiserror::Error;
use valence::prelude::Compound;
use valence::protocol::ident::{Ident, IdentError};

use crate::chunk::ChunkStatus;

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
    #[error("Missing key: {key}")]
    MissingKey { key: String, tag: Option<Compound> },
    #[error("Invalid type: {key}")]
    InvalidType { key: String, tag: Option<Compound> },
}

#[derive(Error, Debug)]
pub enum DataFormatError {
    #[error("Unknown compression scheme: {0}")]
    UnknownCompressionScheme(u8),
    #[error("Invalid chunk size: {0}")]
    InvalidChunkSize(usize),
    #[error("Missing chunk parameter: {key}")]
    MissingChunkNBT {
        key: &'static str,
        tag: Option<Compound>,
    },
    #[error(transparent)]
    IdentityError(#[from] IdentError<String>),
    #[error("Unknown identity: {0}")]
    UnknownType(Ident<String>),
    #[error("Invalid chunk state: {0}")]
    InvalidChunkState(String),
    #[error("Unexpected chunk state: {0}")]
    UnexpectedChunkState(ChunkStatus),
    #[error("Property load error: {name} {value}")]
    PropertyLoadError { name: String, value: String },
    #[error("Invalid chunk palette")]
    InvalidPalette,
}

impl From<IdentError<String>> for Error {
    fn from(err: IdentError<String>) -> Self {
        Self::DataFormatError(DataFormatError::IdentityError(err))
    }
}
