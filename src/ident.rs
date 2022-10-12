//! Resource identifiers.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::str::FromStr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::nbt;
use crate::protocol::{Decode, Encode};

/// A wrapper around a string type `S` which guarantees the wrapped string is a
/// valid resource identifier.
///
/// A resource identifier is a string divided into a "namespace" part and a
/// "path" part. For instance `minecraft:apple` and `valence:frobnicator` are
/// both valid identifiers.
///
/// If the namespace part is left off (the part before and including the colon)
/// the namespace is considered to be "minecraft" for the purposes of equality,
/// ordering, and hashing.
///
/// A string must match the regex `^([a-z0-9_-]+:)?[a-z0-9_\/.-]+$` to be a
/// valid identifier.
#[derive(Copy, Clone, Debug)]
pub struct Ident<S> {
    string: S,
    path_start: usize,
}

impl<S: Borrow<str>> Ident<S> {
    pub fn new(string: S) -> Result<Self, IdentError<S>> {
        let check_namespace = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'))
        };
        let check_path = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '_' | '/' | '.' | '-'))
        };

        let str = string.borrow();

        match str.split_once(':') {
            Some((namespace, path)) if check_namespace(namespace) && check_path(path) => {
                let path_start = namespace.len() + 1;
                Ok(Self { string, path_start })
            }
            None if check_path(str) => Ok(Self {
                string,
                path_start: 0,
            }),
            _ => Err(IdentError(string)),
        }
    }

    /// Returns the namespace part of this resource identifier.
    ///
    /// If the underlying string does not contain a namespace followed by a
    /// ':' character, `"minecraft"` is returned.
    pub fn namespace(&self) -> &str {
        if self.path_start == 0 {
            "minecraft"
        } else {
            &self.string.borrow()[..self.path_start - 1]
        }
    }

    pub fn path(&self) -> &str {
        &self.string.borrow()[self.path_start..]
    }

    /// Returns the underlying string as a `str`.
    pub fn as_str(&self) -> &str {
        self.string.borrow()
    }

    /// Borrows the underlying string and returns it as an `Ident`. This
    /// operation is infallible and no checks need to be performed.
    pub fn as_str_ident(&self) -> Ident<&str> {
        Ident {
            string: self.string.borrow(),
            path_start: self.path_start,
        }
    }

    /// Converts the underlying string to its owned representation and returns
    /// it as an `Ident`. This operation is infallible and no checks need to be
    /// performed.
    pub fn to_owned_ident(&self) -> Ident<S::Owned>
    where
        S: ToOwned,
        S::Owned: Borrow<str>,
    {
        Ident {
            string: self.string.to_owned(),
            path_start: self.path_start,
        }
    }

    /// Consumes the identifier and returns the underlying string.
    pub fn into_inner(self) -> S {
        self.string
    }
}

/// The error type created when an [`Ident`] cannot be parsed from a
/// string. Contains the offending string.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdentError<S>(pub S);

impl<S> fmt::Debug for IdentError<S>
where
    S: Borrow<str>,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("IdentError").field(&self.0.borrow()).finish()
    }
}

impl<S> fmt::Display for IdentError<S>
where
    S: Borrow<str>,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "invalid resource identifier \"{}\"", self.0.borrow())
    }
}

impl<S> Error for IdentError<S> where S: Borrow<str> {}

impl FromStr for Ident<String> {
    type Err = IdentError<String>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ident::new(s.to_owned())
    }
}

impl TryFrom<String> for Ident<String> {
    type Error = IdentError<String>;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ident::new(value)
    }
}

impl<S> From<Ident<S>> for String
where
    S: Into<String> + Borrow<str>,
{
    fn from(id: Ident<S>) -> Self {
        if id.path_start == 0 {
            format!("minecraft:{}", id.string.borrow())
        } else {
            id.string.into()
        }
    }
}

impl<S: Borrow<str>> fmt::Display for Ident<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace(), self.path())
    }
}

impl<S, T> PartialEq<Ident<T>> for Ident<S>
where
    S: Borrow<str>,
    T: Borrow<str>,
{
    fn eq(&self, other: &Ident<T>) -> bool {
        self.namespace() == other.namespace() && self.path() == other.path()
    }
}

impl<S> Eq for Ident<S> where S: Borrow<str> {}

impl<S, T> PartialOrd<Ident<T>> for Ident<S>
where
    S: Borrow<str>,
    T: Borrow<str>,
{
    fn partial_cmp(&self, other: &Ident<T>) -> Option<Ordering> {
        (self.namespace(), self.path()).partial_cmp(&(other.namespace(), other.path()))
    }
}

impl<S> Ord for Ident<S>
where
    S: Borrow<str>,
{
    fn cmp(&self, other: &Self) -> Ordering {
        (self.namespace(), self.path()).cmp(&(other.namespace(), other.path()))
    }
}

impl<S> Hash for Ident<S>
where
    S: Borrow<str>,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.namespace(), self.path()).hash(state);
    }
}

impl<T> Serialize for Ident<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.string.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Ident<T>
where
    T: Deserialize<'de> + Borrow<str>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ident::new(T::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

impl<S: Encode> Encode for Ident<S> {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        self.string.encode(w)
    }
}

impl<S> Decode for Ident<S>
where
    S: Decode + Borrow<str> + Send + Sync + 'static,
{
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(Ident::new(S::decode(r)?)?)
    }
}

impl<S> From<Ident<S>> for nbt::Value
where
    S: Into<nbt::Value>,
{
    fn from(id: Ident<S>) -> Self {
        id.string.into()
    }
}

/// Convenience macro for constructing an [`Ident<String>`] from a format
/// string.
///
/// The arguments to this macro are forwarded to [`std::format`].
///
/// # Panics
///
/// The macro will cause a panic if the formatted string is not a valid resource
/// identifier. See [`Ident`] for more information.
///
/// [`Ident<String>`]: [Ident]
///
/// # Examples
///
/// ```
/// use valence::ident;
///
/// let namespace = "my_namespace";
/// let path = "my_path";
///
/// let id = ident!("{namespace}:{path}");
///
/// assert_eq!(id.namespace(), "my_namespace");
/// assert_eq!(id.path(), "my_path");
/// ```
#[macro_export]
macro_rules! ident {
    ($($arg:tt)*) => {{
        $crate::ident::Ident::new(::std::format!($($arg)*)).unwrap()
    }}
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

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
        ident!("");
    }

    #[test]
    #[should_panic]
    fn parse_invalid_1() {
        ident!(":");
    }

    #[test]
    #[should_panic]
    fn parse_invalid_2() {
        ident!("foo:bar:baz");
    }

    #[test]
    fn equality() {
        assert_eq!(ident!("minecraft:my.identifier"), ident!("my.identifier"));
    }

    #[test]
    fn equal_hash() {
        let mut h1 = DefaultHasher::new();
        ident!("minecraft:my.identifier").hash(&mut h1);

        let mut h2 = DefaultHasher::new();
        ident!("my.identifier").hash(&mut h2);

        assert_eq!(h1.finish(), h2.finish());
    }
}
