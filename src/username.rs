use std::borrow::Borrow;
use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use std::io::Write;
use std::str::FromStr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::protocol::{Decode, Encode};

/// A newtype wrapper around a string type `S` which guarantees the wrapped
/// string meets the criteria for a valid Minecraft username.
///
/// A valid username is 3 to 16 characters long with only ASCII alphanumeric
/// characters. The username must match the regex `^[a-zA-Z0-9_]{3,16}$` to be
/// considered valid.
///
/// # Contract
///
/// The type `S` must meet the following criteria:
/// - All calls to [`AsRef::as_ref`] and [`Borrow::borrow`] while the string is
///   wrapped in `Username` must return the same value.
///
/// # Examples
///
/// ```
/// use valence::prelude::*;
///
/// assert!(Username::new("00a").is_ok());
/// assert!(Username::new("jeb_").is_ok());
///
/// assert!(Username::new("notavalidusername").is_err());
/// assert!(Username::new("NotValid!").is_err());
/// ```
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
pub struct Username<S>(S);

impl<S: AsRef<str>> Username<S> {
    pub fn new(string: S) -> Result<Self, UsernameError<S>> {
        let s = string.as_ref();

        if (3..=16).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            Ok(Self(string))
        } else {
            Err(UsernameError(string))
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    pub fn as_str_username(&self) -> Username<&str> {
        Username(self.0.as_ref())
    }

    pub fn to_owned_username(&self) -> Username<S::Owned>
    where
        S: ToOwned,
        S::Owned: AsRef<str>,
    {
        Username(self.0.to_owned())
    }

    pub fn into_inner(self) -> S {
        self.0
    }
}

impl<S> AsRef<str> for Username<S>
where
    S: AsRef<str>,
{
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<S> Borrow<str> for Username<S>
where
    S: Borrow<str>,
{
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl FromStr for Username<String> {
    type Err = UsernameError<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Username::new(s.to_owned())
    }
}

impl TryFrom<String> for Username<String> {
    type Error = UsernameError<String>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Username::new(value)
    }
}

impl<S> From<Username<S>> for String
where
    S: Into<String> + AsRef<str>,
{
    fn from(value: Username<S>) -> Self {
        value.0.into()
    }
}

impl<S> fmt::Display for Username<S>
where
    S: AsRef<str>,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.as_ref().fmt(f)
    }
}

impl<S> Encode for Username<S>
where
    S: Encode,
{
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        self.0.encode(w)
    }

    fn encoded_len(&self) -> usize {
        self.0.encoded_len()
    }
}

impl<S> Decode for Username<S>
where
    S: Decode + AsRef<str> + Send + Sync + 'static,
{
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(Username::new(S::decode(r)?)?)
    }
}

impl<'de, S> Deserialize<'de> for Username<S>
where
    S: Deserialize<'de> + AsRef<str>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Username::new(S::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// The error type created when a [`Username`] cannot be parsed from a string.
/// Contains the offending string.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UsernameError<S>(pub S);

impl<S> fmt::Debug for UsernameError<S>
where
    S: AsRef<str>,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("UsernameError")
            .field(&self.0.as_ref())
            .finish()
    }
}

impl<S> fmt::Display for UsernameError<S>
where
    S: AsRef<str>,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "invalid username \"{}\"", self.0.as_ref())
    }
}

impl<S> Error for UsernameError<S> where S: AsRef<str> {}
