use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;
use std::iter::FusedIterator;
use std::ops::{Index, IndexMut};

use crate::to_binary_writer::written_size;
use crate::Value;

/// A map type with [`String`] keys and [`Value`] values.
#[derive(Clone, PartialEq, Default)]
pub struct Compound {
    map: Map,
}

#[cfg(not(feature = "preserve_order"))]
type Map = std::collections::BTreeMap<String, Value>;

#[cfg(feature = "preserve_order")]
type Map = indexmap::IndexMap<String, Value>;

impl Compound {
    /// Returns the number of bytes that will be written when
    /// [`to_binary_writer`] is called with this compound and root name.
    ///
    /// If [`to_binary_writer`] results in `Ok`, the exact number of bytes
    /// reported by this function will have been written. If the result is
    /// `Err`, then the reported count will be greater than or equal to the
    /// number of bytes that have actually been written.
    ///
    /// [`to_binary_writer`]: crate::to_binary_writer()
    pub fn written_size(&self, root_name: &str) -> usize {
        written_size(self, root_name)
    }
}

impl fmt::Debug for Compound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.map.fmt(f)
    }
}

impl Compound {
    pub fn new() -> Self {
        Self { map: Map::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            #[cfg(not(feature = "preserve_order"))]
            map: {
                // BTreeMap does not have with_capacity.
                let _ = cap;
                Map::new()
            },
            #[cfg(feature = "preserve_order")]
            map: Map::with_capacity(cap),
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn get<Q>(&self, k: &Q) -> Option<&Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Eq + Ord + Hash,
    {
        self.map.get(k)
    }

    pub fn contains_key<Q>(&self, k: &Q) -> bool
    where
        String: Borrow<Q>,
        Q: ?Sized + Eq + Ord + Hash,
    {
        self.map.contains_key(k)
    }

    pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Eq + Ord + Hash,
    {
        self.map.get_mut(k)
    }

    pub fn get_key_value<Q>(&self, k: &Q) -> Option<(&String, &Value)>
    where
        String: Borrow<Q>,
        Q: ?Sized + Eq + Ord + Hash,
    {
        self.map.get_key_value(k)
    }

    pub fn insert<K, V>(&mut self, k: K, v: V) -> Option<Value>
    where
        K: Into<String>,
        V: Into<Value>,
    {
        self.map.insert(k.into(), v.into())
    }

    pub fn remove<Q>(&mut self, k: &Q) -> Option<Value>
    where
        String: Borrow<Q>,
        Q: ?Sized + Eq + Ord + Hash,
    {
        self.map.remove(k)
    }

    pub fn remove_entry<Q>(&mut self, k: &Q) -> Option<(String, Value)>
    where
        String: Borrow<Q>,
        Q: ?Sized + Eq + Ord + Hash,
    {
        self.map.remove_entry(k)
    }

    pub fn append(&mut self, other: &mut Self) {
        #[cfg(not(feature = "preserve_order"))]
        self.map.append(&mut other.map);

        #[cfg(feature = "preserve_order")]
        for (k, v) in std::mem::take(&mut other.map) {
            self.map.insert(k, v);
        }
    }

    pub fn entry<K>(&mut self, k: K) -> Entry
    where
        K: Into<String>,
    {
        #[cfg(not(feature = "preserve_order"))]
        use std::collections::btree_map::Entry as EntryImpl;

        #[cfg(feature = "preserve_order")]
        use indexmap::map::Entry as EntryImpl;

        match self.map.entry(k.into()) {
            EntryImpl::Vacant(ve) => Entry::Vacant(VacantEntry { ve }),
            EntryImpl::Occupied(oe) => Entry::Occupied(OccupiedEntry { oe }),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> Iter {
        Iter {
            iter: self.map.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            iter: self.map.iter_mut(),
        }
    }

    pub fn keys(&self) -> Keys {
        Keys {
            iter: self.map.keys(),
        }
    }

    pub fn values(&self) -> Values {
        Values {
            iter: self.map.values(),
        }
    }

    pub fn values_mut(&mut self) -> ValuesMut {
        ValuesMut {
            iter: self.map.values_mut(),
        }
    }

    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&String, &mut Value) -> bool,
    {
        self.map.retain(f)
    }
}

impl Extend<(String, Value)> for Compound {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = (String, Value)>,
    {
        self.map.extend(iter)
    }
}

impl FromIterator<(String, Value)> for Compound {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (String, Value)>,
    {
        Self {
            map: Map::from_iter(iter),
        }
    }
}

pub enum Entry<'a> {
    Vacant(VacantEntry<'a>),
    Occupied(OccupiedEntry<'a>),
}

impl<'a> Entry<'a> {
    pub fn key(&self) -> &String {
        match self {
            Entry::Vacant(ve) => ve.key(),
            Entry::Occupied(oe) => oe.key(),
        }
    }

    pub fn or_insert(self, default: impl Into<Value>) -> &'a mut Value {
        match self {
            Entry::Vacant(ve) => ve.insert(default),
            Entry::Occupied(oe) => oe.into_mut(),
        }
    }

    pub fn or_insert_with<F, V>(self, default: F) -> &'a mut Value
    where
        F: FnOnce() -> V,
        V: Into<Value>,
    {
        match self {
            Entry::Vacant(ve) => ve.insert(default()),
            Entry::Occupied(oe) => oe.into_mut(),
        }
    }

    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut Value),
    {
        match self {
            Entry::Vacant(ve) => Entry::Vacant(ve),
            Entry::Occupied(mut oe) => {
                f(oe.get_mut());
                Entry::Occupied(oe)
            }
        }
    }
}

pub struct VacantEntry<'a> {
    #[cfg(not(feature = "preserve_order"))]
    ve: std::collections::btree_map::VacantEntry<'a, String, Value>,
    #[cfg(feature = "preserve_order")]
    ve: indexmap::map::VacantEntry<'a, String, Value>,
}

impl<'a> VacantEntry<'a> {
    pub fn key(&self) -> &String {
        self.ve.key()
    }

    pub fn insert(self, v: impl Into<Value>) -> &'a mut Value {
        self.ve.insert(v.into())
    }
}

pub struct OccupiedEntry<'a> {
    #[cfg(not(feature = "preserve_order"))]
    oe: std::collections::btree_map::OccupiedEntry<'a, String, Value>,
    #[cfg(feature = "preserve_order")]
    oe: indexmap::map::OccupiedEntry<'a, String, Value>,
}

impl<'a> OccupiedEntry<'a> {
    pub fn key(&self) -> &String {
        self.oe.key()
    }

    pub fn get(&self) -> &Value {
        self.oe.get()
    }

    pub fn get_mut(&mut self) -> &mut Value {
        self.oe.get_mut()
    }

    pub fn into_mut(self) -> &'a mut Value {
        self.oe.into_mut()
    }

    pub fn insert(&mut self, v: impl Into<Value>) -> Value {
        self.oe.insert(v.into())
    }

    pub fn remove(self) -> Value {
        self.oe.remove()
    }
}

impl<Q> Index<&'_ Q> for Compound
where
    String: Borrow<Q>,
    Q: ?Sized + Eq + Ord + Hash,
{
    type Output = Value;

    fn index(&self, index: &Q) -> &Self::Output {
        self.map.index(index)
    }
}

impl<Q> IndexMut<&'_ Q> for Compound
where
    String: Borrow<Q>,
    Q: ?Sized + Eq + Ord + Hash,
{
    fn index_mut(&mut self, index: &Q) -> &mut Self::Output {
        self.map.get_mut(index).expect("no entry found for key")
    }
}

macro_rules! impl_iterator_traits {
    (($name:ident $($generics:tt)*) => $item:ty) => {
        impl $($generics)* Iterator for $name $($generics)* {
            type Item = $item;
            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next()
            }
            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.iter.size_hint()
            }
        }

        #[cfg(feature = "preserve_order")]
        impl $($generics)* DoubleEndedIterator for $name $($generics)* {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                self.iter.next_back()
            }
        }

        impl $($generics)* ExactSizeIterator for $name $($generics)* {
            #[inline]
            fn len(&self) -> usize {
                self.iter.len()
            }
        }

        impl $($generics)* FusedIterator for $name $($generics)* {}
    }
}

impl<'a> IntoIterator for &'a Compound {
    type Item = (&'a String, &'a Value);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
            iter: self.map.iter(),
        }
    }
}

#[derive(Clone)]
pub struct Iter<'a> {
    #[cfg(not(feature = "preserve_order"))]
    iter: std::collections::btree_map::Iter<'a, String, Value>,
    #[cfg(feature = "preserve_order")]
    iter: indexmap::map::Iter<'a, String, Value>,
}

impl_iterator_traits!((Iter<'a>) => (&'a String, &'a Value));

impl<'a> IntoIterator for &'a mut Compound {
    type Item = (&'a String, &'a mut Value);
    type IntoIter = IterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        IterMut {
            iter: self.map.iter_mut(),
        }
    }
}

pub struct IterMut<'a> {
    #[cfg(not(feature = "preserve_order"))]
    iter: std::collections::btree_map::IterMut<'a, String, Value>,
    #[cfg(feature = "preserve_order")]
    iter: indexmap::map::IterMut<'a, String, Value>,
}

impl_iterator_traits!((IterMut<'a>) => (&'a String, &'a mut Value));

impl IntoIterator for Compound {
    type Item = (String, Value);
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            iter: self.map.into_iter(),
        }
    }
}

pub struct IntoIter {
    #[cfg(not(feature = "preserve_order"))]
    iter: std::collections::btree_map::IntoIter<String, Value>,
    #[cfg(feature = "preserve_order")]
    iter: indexmap::map::IntoIter<String, Value>,
}

impl_iterator_traits!((IntoIter) => (String, Value));

#[derive(Clone)]
pub struct Keys<'a> {
    #[cfg(not(feature = "preserve_order"))]
    iter: std::collections::btree_map::Keys<'a, String, Value>,
    #[cfg(feature = "preserve_order")]
    iter: indexmap::map::Keys<'a, String, Value>,
}

impl_iterator_traits!((Keys<'a>) => &'a String);

#[derive(Clone)]
pub struct Values<'a> {
    #[cfg(not(feature = "preserve_order"))]
    iter: std::collections::btree_map::Values<'a, String, Value>,
    #[cfg(feature = "preserve_order")]
    iter: indexmap::map::Values<'a, String, Value>,
}

impl_iterator_traits!((Values<'a>) => &'a Value);

pub struct ValuesMut<'a> {
    #[cfg(not(feature = "preserve_order"))]
    iter: std::collections::btree_map::ValuesMut<'a, String, Value>,
    #[cfg(feature = "preserve_order")]
    iter: indexmap::map::ValuesMut<'a, String, Value>,
}

impl_iterator_traits!((ValuesMut<'a>) => &'a mut Value);
