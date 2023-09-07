use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Copy, Eq, PartialEq, Clone, Debug)]
pub struct Utf8Error {
    pub(crate) valid_up_to: usize,
    pub(crate) error_len: Option<u8>,
}

impl Utf8Error {
    #[must_use]
    #[inline]
    pub const fn valid_up_to(&self) -> usize {
        self.valid_up_to
    }

    #[must_use]
    #[inline]
    pub const fn error_len(&self) -> Option<usize> {
        // Manual implementation of Option::map since it's not const
        match self.error_len {
            Some(len) => Some(len as usize),
            None => None,
        }
    }

    #[must_use]
    #[inline]
    pub(crate) const fn from_std(value: std::str::Utf8Error) -> Self {
        Self {
            valid_up_to: value.valid_up_to(),
            // Manual implementation of Option::map since it's not const
            error_len: match value.error_len() {
                Some(error_len) => Some(error_len as u8),
                None => None,
            },
        }
    }
}

impl Display for Utf8Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(error_len) = self.error_len {
            write!(
                f,
                "invalid utf-8 sequence of {} bytes from index {}",
                error_len, self.valid_up_to
            )
        } else {
            write!(
                f,
                "incomplete utf-8 byte sequence from index {}",
                self.valid_up_to
            )
        }
    }
}

impl From<std::str::Utf8Error> for Utf8Error {
    #[inline]
    fn from(value: std::str::Utf8Error) -> Self {
        Self::from_std(value)
    }
}

impl Error for Utf8Error {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FromUtf8Error {
    pub(crate) bytes: Vec<u8>,
    pub(crate) error: Utf8Error,
}

impl FromUtf8Error {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..]
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub fn utf8_error(&self) -> Utf8Error {
        self.error
    }
}

impl Display for FromUtf8Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.error, f)
    }
}

impl Error for FromUtf8Error {}

#[derive(Copy, Eq, PartialEq, Clone, Debug)]
pub enum ParseError<E> {
    InvalidUtf8(Utf8Error),
    Err(E),
}

impl<E> Display for ParseError<E>
where
    E: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::InvalidUtf8(err) => Display::fmt(err, f),
            ParseError::Err(err) => Display::fmt(err, f),
        }
    }
}

impl<E> Error for ParseError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseError::InvalidUtf8(err) => Some(err),
            ParseError::Err(err) => Some(err),
        }
    }
}
