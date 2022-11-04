use std::fmt::{Display, Formatter};
use std::{fmt, io};

use thiserror::Error;
use valence::ident::{Ident, IdentError};

#[derive(Debug, Error)]
pub enum Error {
    Io(io::Error),
    DataFormatError(DataFormatError),
    NbtParseError(valence::nbt::Error),
    NbtFormatError(NbtFormatError),
}

#[derive(Debug)]
pub enum NbtFormatError {
    MissingKey(String),
    InvalidType(String),
}

#[derive(Debug)]
pub enum DataFormatError {
    UnknownCompressionScheme(u8),
    InvalidChunkSize(usize),
    IdentityError(IdentError<String>),
    UnknownType(Ident<String>),
    InvalidChunkState(String),
    InvalidPalette,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<valence::nbt::Error> for Error {
    fn from(e: valence::nbt::Error) -> Self {
        Self::NbtParseError(e)
    }
}

impl From<IdentError<String>> for Error {
    fn from(e: IdentError<String>) -> Self {
        Self::DataFormatError(DataFormatError::IdentityError(e))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => e.fmt(f),
            Error::DataFormatError(e) => e.fmt(f),
            Error::NbtParseError(e) => e.fmt(f),
            Error::NbtFormatError(e) => e.fmt(f),
        }
    }
}

impl Display for DataFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DataFormatError::UnknownCompressionScheme(scheme) => {
                write!(f, "Unknown compression scheme: {scheme}")
            }
            DataFormatError::InvalidChunkSize(size) => write!(f, "Invalid chunk size: {size}"),
            DataFormatError::IdentityError(e) => e.fmt(f),
            DataFormatError::UnknownType(identity) => write!(f, "Unknown identity: {identity}"),
            DataFormatError::InvalidChunkState(state) => write!(f, "Unknown chunk state: {state}"),
            DataFormatError::InvalidPalette => write!(f, "Invalid chunk palette"),
        }
    }
}

impl Display for NbtFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            NbtFormatError::MissingKey(key) => {
                write!(f, "Could not find key: \"{key}\" in nbt data.")
            }
            NbtFormatError::InvalidType(key) => write!(f, "Unexpected type for key: \"{key}\""),
        }
    }
}
