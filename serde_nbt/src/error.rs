use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::io;

use anyhow::anyhow;
use serde::{de, ser};

#[derive(Debug)]
pub struct Error(pub(super) anyhow::Error);

impl Error {
    pub(super) fn context<C>(self, ctx: C) -> Self
    where
        C: Display + Send + Sync + 'static,
    {
        Self(self.0.context(ctx))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }
}

impl ser::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error(anyhow!("{msg}"))
    }
}

impl de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Error(anyhow!("{msg}"))
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error(anyhow::Error::new(e))
    }
}
