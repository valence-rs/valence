//! Resource identifiers.

use std::borrow::{Borrow, Cow};
use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::io::Write;
use std::str::FromStr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::packet::{Decode, Encode};

#[doc(hidden)]
pub mod __private {
    pub use valence_core_macros::parse_ident_str;
}

/// A wrapper around a string type `S` which guarantees the wrapped string is a
/// valid resource identifier.
///
/// A resource identifier is a string divided into a "namespace" part and a
/// "path" part. For instance `minecraft:apple` and `valence:frobnicator` are
/// both valid identifiers. A string must match the regex
/// `^([a-z0-9_.-]+:)?[a-z0-9_.-\/]+$` to be successfully parsed.
///
/// While parsing, if the namespace part is left off (the part before and
/// including the colon) then "minecraft:" is inserted at the beginning of the
/// string.
#[derive(Copy, Clone, Eq, Ord, Hash)]
pub struct Ident<S> {
    string: S,
}

/// Creates a new [`Ident`] at compile time from a string literal. A compile
/// error is raised if the string is not a valid resource identifier.
///
/// The type of the expression returned by this macro is `Ident<&'static str>`.
///
/// # Examples
///
/// ```
/// # use valence_core::{ident, ident::Ident};
/// let my_ident: Ident<&'static str> = ident!("apple");
///
/// println!("{my_ident}");
/// ```
#[macro_export]
macro_rules! ident {
    ($string:literal) => {
        $crate::ident::Ident::<&'static str>::new_unchecked(
            $crate::ident::__private::parse_ident_str!($string),
        )
    };
}

/// The error type created when an [`Ident`] cannot be parsed from a
/// string. Contains the string that failed to parse.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Error)]
#[error("invalid resource identifier \"{0}\"")]
pub struct IdentError(pub String);

impl<'a> Ident<Cow<'a, str>> {
    pub fn new(string: impl Into<Cow<'a, str>>) -> Result<Self, IdentError> {
        parse(string.into())
    }
}

impl<S> Ident<S> {
    /// Internal API. Do not use.
    #[doc(hidden)]
    pub const fn new_unchecked(string: S) -> Self {
        Self { string }
    }

    pub fn as_str(&self) -> &str
    where
        S: AsRef<str>,
    {
        self.string.as_ref()
    }

    pub fn as_str_ident(&self) -> Ident<&str>
    where
        S: AsRef<str>,
    {
        Ident {
            string: self.as_str(),
        }
    }

    pub fn to_string_ident(&self) -> Ident<String>
    where
        S: AsRef<str>,
    {
        Ident {
            string: self.as_str().to_owned(),
        }
    }

    pub fn into_inner(self) -> S {
        self.string
    }

    /// Returns the namespace part of this resource identifier (the part before
    /// the colon).
    pub fn namespace(&self) -> &str
    where
        S: AsRef<str>,
    {
        self.namespace_and_path().0
    }

    /// Returns the path part of this resource identifier (the part after the
    /// colon).
    pub fn path(&self) -> &str
    where
        S: AsRef<str>,
    {
        self.namespace_and_path().1
    }

    pub fn namespace_and_path(&self) -> (&str, &str)
    where
        S: AsRef<str>,
    {
        self.as_str()
            .split_once(':')
            .expect("invalid resource identifier")
    }
}

fn parse(string: Cow<str>) -> Result<Ident<Cow<str>>, IdentError> {
    let check_namespace = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '.' | '-'))
    };

    let check_path = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '.' | '-' | '/'))
    };

    match string.split_once(':') {
        Some((namespace, path)) if check_namespace(namespace) && check_path(path) => {
            Ok(Ident { string })
        }
        None if check_path(&string) => Ok(Ident {
            string: format!("minecraft:{string}").into(),
        }),
        _ => Err(IdentError(string.into())),
    }
}

impl<S: AsRef<str>> AsRef<str> for Ident<S> {
    fn as_ref(&self) -> &str {
        self.string.as_ref()
    }
}

impl<S> AsRef<S> for Ident<S> {
    fn as_ref(&self) -> &S {
        &self.string
    }
}

impl<S: Borrow<str>> Borrow<str> for Ident<S> {
    fn borrow(&self) -> &str {
        self.string.borrow()
    }
}

impl From<Ident<&str>> for String {
    fn from(value: Ident<&str>) -> Self {
        value.as_str().to_owned()
    }
}

impl From<Ident<String>> for String {
    fn from(value: Ident<String>) -> Self {
        value.into_inner()
    }
}

impl<'a> From<Ident<Cow<'a, str>>> for Cow<'a, str> {
    fn from(value: Ident<Cow<'a, str>>) -> Self {
        value.into_inner()
    }
}

impl<'a> From<Ident<Cow<'a, str>>> for Ident<String> {
    fn from(value: Ident<Cow<'a, str>>) -> Self {
        Self {
            string: value.string.into(),
        }
    }
}

impl<'a> From<Ident<String>> for Ident<Cow<'a, str>> {
    fn from(value: Ident<String>) -> Self {
        Self {
            string: value.string.into(),
        }
    }
}

impl<'a> From<Ident<&'a str>> for Ident<Cow<'a, str>> {
    fn from(value: Ident<&'a str>) -> Self {
        Ident {
            string: value.string.into(),
        }
    }
}

impl<'a> From<Ident<&'a str>> for Ident<String> {
    fn from(value: Ident<&'a str>) -> Self {
        Ident {
            string: value.string.into(),
        }
    }
}

impl FromStr for Ident<String> {
    type Err = IdentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Ident::new(s)?.into())
    }
}

impl FromStr for Ident<Cow<'static, str>> {
    type Err = IdentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ident::<String>::try_from(s).map(From::from)
    }
}

impl<'a> TryFrom<&'a str> for Ident<String> {
    type Error = IdentError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Ok(Ident::new(value)?.into())
    }
}

impl TryFrom<String> for Ident<String> {
    type Error = IdentError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Ident::new(value)?.into())
    }
}

impl<'a> TryFrom<Cow<'a, str>> for Ident<String> {
    type Error = IdentError;

    fn try_from(value: Cow<'a, str>) -> Result<Self, Self::Error> {
        Ok(Ident::new(value)?.into())
    }
}

impl<'a> TryFrom<&'a str> for Ident<Cow<'a, str>> {
    type Error = IdentError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> TryFrom<String> for Ident<Cow<'a, str>> {
    type Error = IdentError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> TryFrom<Cow<'a, str>> for Ident<Cow<'a, str>> {
    type Error = IdentError;

    fn try_from(value: Cow<'a, str>) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<S: fmt::Debug> fmt::Debug for Ident<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.string.fmt(f)
    }
}

impl<S: fmt::Display> fmt::Display for Ident<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.string.fmt(f)
    }
}

impl<S, T> PartialEq<Ident<T>> for Ident<S>
where
    S: PartialEq<T>,
{
    fn eq(&self, other: &Ident<T>) -> bool {
        self.string == other.string
    }
}

impl<S, T> PartialOrd<Ident<T>> for Ident<S>
where
    S: PartialOrd<T>,
{
    fn partial_cmp(&self, other: &Ident<T>) -> Option<Ordering> {
        self.string.partial_cmp(&other.string)
    }
}

impl<S: Encode> Encode for Ident<S> {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.as_ref().encode(w)
    }
}

impl<'a, S> Decode<'a> for Ident<S>
where
    S: Decode<'a>,
    Ident<S>: TryFrom<S, Error = IdentError>,
{
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(Ident::try_from(S::decode(r)?)?)
    }
}

impl<S> From<Ident<S>> for valence_nbt::Value
where
    S: Into<valence_nbt::Value>,
{
    fn from(value: Ident<S>) -> Self {
        value.into_inner().into()
    }
}

impl<T: Serialize> Serialize for Ident<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.string.serialize(serializer)
    }
}

impl<'de, S> Deserialize<'de> for Ident<S>
where
    S: Deserialize<'de>,
    Ident<S>: TryFrom<S, Error = IdentError>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ident::try_from(S::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_namespace_and_path() {
        let id = ident!("namespace:path");
        assert_eq!(id.namespace(), "namespace");
        assert_eq!(id.path(), "path");
    }

    #[test]
    fn parse_valid() {
        ident!("minecraft:whatever");
        ident!("_what-ever55_:.whatever/whatever123456789_");
        ident!("valence:frobnicator");
    }

    #[test]
    #[should_panic]
    fn parse_invalid_0() {
        Ident::new("").unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_invalid_1() {
        Ident::new(":").unwrap();
    }

    #[test]
    #[should_panic]
    fn parse_invalid_2() {
        Ident::new("foo:bar:baz").unwrap();
    }

    #[test]
    fn equality() {
        assert_eq!(ident!("minecraft:my.identifier"), ident!("my.identifier"));
    }
}
