use std::borrow::Cow;
use std::hash::Hash;
use std::iter::FusedIterator;

use crate::value::{ValueMut, ValueRef};
use crate::{Compound, Tag, Value};

/// An NBT list value.
///
/// NBT lists are homogeneous, meaning each list element must be of the same
/// type. This is opposed to a format like JSON where lists can be
/// heterogeneous. Here is a JSON list that would be illegal in NBT:
///
/// ```json
/// [42, "hello", {}]
/// ```
///
/// Every possible element type has its own variant in this enum. As a result,
/// heterogeneous lists are unrepresentable.
#[derive(Clone, Default, Debug)]
pub enum List<S = String> {
    /// The list with the element type of `TAG_End` and length of zero.
    #[default]
    End,
    Byte(Vec<i8>),
    Short(Vec<i16>),
    Int(Vec<i32>),
    Long(Vec<i64>),
    Float(Vec<f32>),
    Double(Vec<f64>),
    ByteArray(Vec<Vec<i8>>),
    String(Vec<S>),
    List(Vec<List<S>>),
    Compound(Vec<Compound<S>>),
    IntArray(Vec<Vec<i32>>),
    LongArray(Vec<Vec<i64>>),
}

impl<S> PartialEq for List<S>
where
    S: Ord + Hash,
{
    fn eq(&self, other: &Self) -> bool {
        match self {
            List::End => matches!(other, List::End),
            List::Byte(list) => matches!(other, List::Byte(other_list) if list == other_list),
            List::Short(list) => matches!(other, List::Short(other_list) if list == other_list),
            List::Int(list) => matches!(other, List::Int(other_list) if list == other_list),
            List::Long(list) => matches!(other, List::Long(other_list) if list == other_list),
            List::Float(list) => matches!(other, List::Float(other_list) if list == other_list),
            List::Double(list) => matches!(other, List::Double(other_list) if list == other_list),
            List::ByteArray(list) => {
                matches!(other, List::ByteArray(other_list) if list == other_list)
            }
            List::String(list) => matches!(other, List::String(other_list) if list == other_list),
            List::List(list) => matches!(other, List::List(other_list) if list == other_list),
            List::Compound(list) => {
                matches!(other, List::Compound(other_list) if list == other_list)
            }
            List::IntArray(list) => {
                matches!(other, List::IntArray(other_list) if list == other_list)
            }
            List::LongArray(list) => {
                matches!(other, List::LongArray(other_list) if list == other_list)
            }
        }
    }
}

impl<S> List<S> {
    /// Constructs a new empty NBT list, with the element type of `TAG_End`.
    pub fn new() -> Self {
        Self::End
    }

    /// Returns the length of this list.
    pub fn len(&self) -> usize {
        match self {
            List::End => 0,
            List::Byte(l) => l.len(),
            List::Short(l) => l.len(),
            List::Int(l) => l.len(),
            List::Long(l) => l.len(),
            List::Float(l) => l.len(),
            List::Double(l) => l.len(),
            List::ByteArray(l) => l.len(),
            List::String(l) => l.len(),
            List::List(l) => l.len(),
            List::Compound(l) => l.len(),
            List::IntArray(l) => l.len(),
            List::LongArray(l) => l.len(),
        }
    }

    /// Returns `true` if this list has no elements. `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the element type of this list.
    pub fn element_tag(&self) -> Tag {
        match self {
            List::End => Tag::End,
            List::Byte(_) => Tag::Byte,
            List::Short(_) => Tag::Short,
            List::Int(_) => Tag::Int,
            List::Long(_) => Tag::Long,
            List::Float(_) => Tag::Float,
            List::Double(_) => Tag::Double,
            List::ByteArray(_) => Tag::ByteArray,
            List::String(_) => Tag::String,
            List::List(_) => Tag::List,
            List::Compound(_) => Tag::Compound,
            List::IntArray(_) => Tag::IntArray,
            List::LongArray(_) => Tag::LongArray,
        }
    }

    /// Gets a reference to the value at the given index in this list, or `None`
    /// if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<ValueRef<S>> {
        match self {
            List::End => None,
            List::Byte(list) => list.get(index).map(ValueRef::Byte),
            List::Short(list) => list.get(index).map(ValueRef::Short),
            List::Int(list) => list.get(index).map(ValueRef::Int),
            List::Long(list) => list.get(index).map(ValueRef::Long),
            List::Float(list) => list.get(index).map(ValueRef::Float),
            List::Double(list) => list.get(index).map(ValueRef::Double),
            List::ByteArray(list) => list.get(index).map(|arr| ValueRef::ByteArray(&arr[..])),
            List::String(list) => list.get(index).map(ValueRef::String),
            List::List(list) => list.get(index).map(ValueRef::List),
            List::Compound(list) => list.get(index).map(ValueRef::Compound),
            List::IntArray(list) => list.get(index).map(|arr| ValueRef::IntArray(&arr[..])),
            List::LongArray(list) => list.get(index).map(|arr| ValueRef::LongArray(&arr[..])),
        }
    }

    /// Gets a mutable reference to the value at the given index in this list,
    /// or `None` if the index is out of bounds.
    pub fn get_mut(&mut self, index: usize) -> Option<ValueMut<S>> {
        match self {
            List::End => None,
            List::Byte(list) => list.get_mut(index).map(ValueMut::Byte),
            List::Short(list) => list.get_mut(index).map(ValueMut::Short),
            List::Int(list) => list.get_mut(index).map(ValueMut::Int),
            List::Long(list) => list.get_mut(index).map(ValueMut::Long),
            List::Float(list) => list.get_mut(index).map(ValueMut::Float),
            List::Double(list) => list.get_mut(index).map(ValueMut::Double),
            List::ByteArray(list) => list.get_mut(index).map(ValueMut::ByteArray),
            List::String(list) => list.get_mut(index).map(ValueMut::String),
            List::List(list) => list.get_mut(index).map(ValueMut::List),
            List::Compound(list) => list.get_mut(index).map(ValueMut::Compound),
            List::IntArray(list) => list.get_mut(index).map(ValueMut::IntArray),
            List::LongArray(list) => list.get_mut(index).map(ValueMut::LongArray),
        }
    }

    /// Attempts to add the given value to the end of this list, failing if
    /// adding the value would result in the list not being heterogeneous (have
    /// multiple types inside it). Returns `true` if the value was added,
    /// `false` otherwise.
    #[must_use]
    pub fn try_push(&mut self, value: impl Into<Value<S>>) -> bool {
        let value = value.into();
        match self {
            List::End => {
                *self = List::from(value);
                true
            }
            List::Byte(list) => {
                if let Value::Byte(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::Short(list) => {
                if let Value::Short(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::Int(list) => {
                if let Value::Int(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::Long(list) => {
                if let Value::Long(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::Float(list) => {
                if let Value::Float(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::Double(list) => {
                if let Value::Double(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::ByteArray(list) => {
                if let Value::ByteArray(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::String(list) => {
                if let Value::String(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::List(list) => {
                if let Value::List(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::Compound(list) => {
                if let Value::Compound(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::IntArray(list) => {
                if let Value::IntArray(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
            List::LongArray(list) => {
                if let Value::LongArray(value) = value {
                    list.push(value);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Attempts to insert the given value at the given index in this list,
    /// failing if adding the value would result in the list not being
    /// heterogeneous (have multiple types inside it). Returns `true` if the
    /// value was added, `false` otherwise.
    ///
    /// # Panics
    ///
    /// Panics if the index is greater than the length of the list.
    #[must_use]
    pub fn try_insert(&mut self, index: usize, value: impl Into<Value<S>>) -> bool {
        let value = value.into();

        #[cold]
        #[inline(never)]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("insertion index (is {index}) should be <= len (is {len})");
        }

        match self {
            List::End => {
                if index > 0 {
                    assert_failed(index, 0);
                }
                *self = List::from(value);
                true
            }
            List::Byte(list) => {
                if let Value::Byte(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::Short(list) => {
                if let Value::Short(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::Int(list) => {
                if let Value::Int(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::Long(list) => {
                if let Value::Long(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::Float(list) => {
                if let Value::Float(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::Double(list) => {
                if let Value::Double(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::ByteArray(list) => {
                if let Value::ByteArray(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::String(list) => {
                if let Value::String(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::List(list) => {
                if let Value::List(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::Compound(list) => {
                if let Value::Compound(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::IntArray(list) => {
                if let Value::IntArray(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
            List::LongArray(list) => {
                if let Value::LongArray(value) = value {
                    list.insert(index, value);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Removes the element at the given index in the list, and returns the
    /// value removed.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    #[track_caller]
    pub fn remove(&mut self, index: usize) -> Value<S> {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("removal index (is {index}) should be < len (is {len})");
        }

        let removed = match self {
            List::End => assert_failed(index, 0),
            List::Byte(list) => Value::Byte(list.remove(index)),
            List::Short(list) => Value::Short(list.remove(index)),
            List::Int(list) => Value::Int(list.remove(index)),
            List::Long(list) => Value::Long(list.remove(index)),
            List::Float(list) => Value::Float(list.remove(index)),
            List::Double(list) => Value::Double(list.remove(index)),
            List::ByteArray(list) => Value::ByteArray(list.remove(index)),
            List::String(list) => Value::String(list.remove(index)),
            List::List(list) => Value::List(list.remove(index)),
            List::Compound(list) => Value::Compound(list.remove(index)),
            List::IntArray(list) => Value::IntArray(list.remove(index)),
            List::LongArray(list) => Value::LongArray(list.remove(index)),
        };

        if self.is_empty() {
            *self = List::End;
        }

        removed
    }

    /// Returns only the elements specified by the predicate, passing a mutable
    /// reference to it.
    ///
    /// In other words, removes all elements `e` such that `f(ValueMut(&mut e))`
    /// returns `false`. This method operates in place, visiting each element
    /// exactly once in the original order, and preserves the order of the
    /// retained elements.
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(ValueMut<S>) -> bool,
    {
        match self {
            List::End => {}
            List::Byte(list) => list.retain_mut(|v| f(ValueMut::Byte(v))),
            List::Short(list) => list.retain_mut(|v| f(ValueMut::Short(v))),
            List::Int(list) => list.retain_mut(|v| f(ValueMut::Int(v))),
            List::Long(list) => list.retain_mut(|v| f(ValueMut::Long(v))),
            List::Float(list) => list.retain_mut(|v| f(ValueMut::Float(v))),
            List::Double(list) => list.retain_mut(|v| f(ValueMut::Double(v))),
            List::ByteArray(list) => list.retain_mut(|v| f(ValueMut::ByteArray(v))),
            List::String(list) => list.retain_mut(|v| f(ValueMut::String(v))),
            List::List(list) => list.retain_mut(|v| f(ValueMut::List(v))),
            List::Compound(list) => list.retain_mut(|v| f(ValueMut::Compound(v))),
            List::IntArray(list) => list.retain_mut(|v| f(ValueMut::IntArray(v))),
            List::LongArray(list) => list.retain_mut(|v| f(ValueMut::LongArray(v))),
        }

        if self.is_empty() {
            *self = List::End;
        }
    }

    /// Returns an iterator over this list. This iterator yields [`ValueRef`]s.
    pub fn iter(&self) -> Iter<S> {
        Iter {
            inner: match self {
                List::End => IterInner::End,
                List::Byte(list) => IterInner::Byte(list.iter()),
                List::Short(list) => IterInner::Short(list.iter()),
                List::Int(list) => IterInner::Int(list.iter()),
                List::Long(list) => IterInner::Long(list.iter()),
                List::Float(list) => IterInner::Float(list.iter()),
                List::Double(list) => IterInner::Double(list.iter()),
                List::ByteArray(list) => IterInner::ByteArray(list.iter()),
                List::String(list) => IterInner::String(list.iter()),
                List::List(list) => IterInner::List(list.iter()),
                List::Compound(list) => IterInner::Compound(list.iter()),
                List::IntArray(list) => IterInner::IntArray(list.iter()),
                List::LongArray(list) => IterInner::LongArray(list.iter()),
            },
        }
    }

    /// Returns a mutable iterator over this list. This iterator yields
    /// [`ValueMut`]s.
    pub fn iter_mut(&mut self) -> IterMut<S> {
        IterMut {
            inner: match self {
                List::End => IterMutInner::End,
                List::Byte(list) => IterMutInner::Byte(list.iter_mut()),
                List::Short(list) => IterMutInner::Short(list.iter_mut()),
                List::Int(list) => IterMutInner::Int(list.iter_mut()),
                List::Long(list) => IterMutInner::Long(list.iter_mut()),
                List::Float(list) => IterMutInner::Float(list.iter_mut()),
                List::Double(list) => IterMutInner::Double(list.iter_mut()),
                List::ByteArray(list) => IterMutInner::ByteArray(list.iter_mut()),
                List::String(list) => IterMutInner::String(list.iter_mut()),
                List::List(list) => IterMutInner::List(list.iter_mut()),
                List::Compound(list) => IterMutInner::Compound(list.iter_mut()),
                List::IntArray(list) => IterMutInner::IntArray(list.iter_mut()),
                List::LongArray(list) => IterMutInner::LongArray(list.iter_mut()),
            },
        }
    }
}

impl<S> From<Vec<i8>> for List<S> {
    fn from(v: Vec<i8>) -> Self {
        List::Byte(v)
    }
}

impl<S> From<Vec<i16>> for List<S> {
    fn from(v: Vec<i16>) -> Self {
        List::Short(v)
    }
}

impl<S> From<Vec<i32>> for List<S> {
    fn from(v: Vec<i32>) -> Self {
        List::Int(v)
    }
}

impl<S> From<Vec<i64>> for List<S> {
    fn from(v: Vec<i64>) -> Self {
        List::Long(v)
    }
}

impl<S> From<Vec<f32>> for List<S> {
    fn from(v: Vec<f32>) -> Self {
        List::Float(v)
    }
}

impl<S> From<Vec<f64>> for List<S> {
    fn from(v: Vec<f64>) -> Self {
        List::Double(v)
    }
}

impl<S> From<Vec<Vec<i8>>> for List<S> {
    fn from(v: Vec<Vec<i8>>) -> Self {
        List::ByteArray(v)
    }
}

impl From<Vec<String>> for List<String> {
    fn from(v: Vec<String>) -> Self {
        List::String(v)
    }
}

impl<'a> From<Vec<Cow<'a, str>>> for List<Cow<'a, str>> {
    fn from(v: Vec<Cow<'a, str>>) -> Self {
        List::String(v)
    }
}

#[cfg(feature = "java_string")]
impl From<Vec<java_string::JavaString>> for List<java_string::JavaString> {
    fn from(v: Vec<java_string::JavaString>) -> Self {
        List::String(v)
    }
}

#[cfg(feature = "java_string")]
impl<'a> From<Vec<Cow<'a, java_string::JavaStr>>> for List<Cow<'a, java_string::JavaStr>> {
    fn from(v: Vec<Cow<'a, java_string::JavaStr>>) -> Self {
        List::String(v)
    }
}

impl<S> From<Vec<List<S>>> for List<S> {
    fn from(v: Vec<List<S>>) -> Self {
        List::List(v)
    }
}

impl<S> From<Vec<Compound<S>>> for List<S> {
    fn from(v: Vec<Compound<S>>) -> Self {
        List::Compound(v)
    }
}

impl<S> From<Vec<Vec<i32>>> for List<S> {
    fn from(v: Vec<Vec<i32>>) -> Self {
        List::IntArray(v)
    }
}

impl<S> From<Vec<Vec<i64>>> for List<S> {
    fn from(v: Vec<Vec<i64>>) -> Self {
        List::LongArray(v)
    }
}

/// Converts a value to a singleton list.
impl<S> From<Value<S>> for List<S> {
    fn from(value: Value<S>) -> Self {
        match value {
            Value::Byte(v) => List::Byte(vec![v]),
            Value::Short(v) => List::Short(vec![v]),
            Value::Int(v) => List::Int(vec![v]),
            Value::Long(v) => List::Long(vec![v]),
            Value::Float(v) => List::Float(vec![v]),
            Value::Double(v) => List::Double(vec![v]),
            Value::ByteArray(v) => List::ByteArray(vec![v]),
            Value::String(v) => List::String(vec![v]),
            Value::List(v) => List::List(vec![v]),
            Value::Compound(v) => List::Compound(vec![v]),
            Value::IntArray(v) => List::IntArray(vec![v]),
            Value::LongArray(v) => List::LongArray(vec![v]),
        }
    }
}

impl<S> IntoIterator for List<S> {
    type Item = Value<S>;
    type IntoIter = IntoIter<S>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: match self {
                List::End => IntoIterInner::End,
                List::Byte(list) => IntoIterInner::Byte(list.into_iter()),
                List::Short(list) => IntoIterInner::Short(list.into_iter()),
                List::Int(list) => IntoIterInner::Int(list.into_iter()),
                List::Long(list) => IntoIterInner::Long(list.into_iter()),
                List::Float(list) => IntoIterInner::Float(list.into_iter()),
                List::Double(list) => IntoIterInner::Double(list.into_iter()),
                List::ByteArray(list) => IntoIterInner::ByteArray(list.into_iter()),
                List::String(list) => IntoIterInner::String(list.into_iter()),
                List::List(list) => IntoIterInner::List(list.into_iter()),
                List::Compound(list) => IntoIterInner::Compound(list.into_iter()),
                List::IntArray(list) => IntoIterInner::IntArray(list.into_iter()),
                List::LongArray(list) => IntoIterInner::LongArray(list.into_iter()),
            },
        }
    }
}

impl<'a, S> IntoIterator for &'a List<S> {
    type Item = ValueRef<'a, S>;
    type IntoIter = Iter<'a, S>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, S> IntoIterator for &'a mut List<S> {
    type Item = ValueMut<'a, S>;
    type IntoIter = IterMut<'a, S>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// The owned iterator type for [`List`].
pub struct IntoIter<S = String> {
    inner: IntoIterInner<S>,
}

enum IntoIterInner<S> {
    End,
    Byte(std::vec::IntoIter<i8>),
    Short(std::vec::IntoIter<i16>),
    Int(std::vec::IntoIter<i32>),
    Long(std::vec::IntoIter<i64>),
    Float(std::vec::IntoIter<f32>),
    Double(std::vec::IntoIter<f64>),
    ByteArray(std::vec::IntoIter<Vec<i8>>),
    String(std::vec::IntoIter<S>),
    List(std::vec::IntoIter<List<S>>),
    Compound(std::vec::IntoIter<Compound<S>>),
    IntArray(std::vec::IntoIter<Vec<i32>>),
    LongArray(std::vec::IntoIter<Vec<i64>>),
}

impl<S> Iterator for IntoIter<S> {
    type Item = Value<S>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            IntoIterInner::End => None,
            IntoIterInner::Byte(ref mut i) => i.next().map(Value::Byte),
            IntoIterInner::Short(ref mut i) => i.next().map(Value::Short),
            IntoIterInner::Int(ref mut i) => i.next().map(Value::Int),
            IntoIterInner::Long(ref mut i) => i.next().map(Value::Long),
            IntoIterInner::Float(ref mut i) => i.next().map(Value::Float),
            IntoIterInner::Double(ref mut i) => i.next().map(Value::Double),
            IntoIterInner::ByteArray(ref mut i) => i.next().map(Value::ByteArray),
            IntoIterInner::String(ref mut i) => i.next().map(Value::String),
            IntoIterInner::List(ref mut i) => i.next().map(Value::List),
            IntoIterInner::Compound(ref mut i) => i.next().map(Value::Compound),
            IntoIterInner::IntArray(ref mut i) => i.next().map(Value::IntArray),
            IntoIterInner::LongArray(ref mut i) => i.next().map(Value::LongArray),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.inner {
            IntoIterInner::End => (0, Some(0)),
            IntoIterInner::Byte(ref i) => i.size_hint(),
            IntoIterInner::Short(ref i) => i.size_hint(),
            IntoIterInner::Int(ref i) => i.size_hint(),
            IntoIterInner::Long(ref i) => i.size_hint(),
            IntoIterInner::Float(ref i) => i.size_hint(),
            IntoIterInner::Double(ref i) => i.size_hint(),
            IntoIterInner::ByteArray(ref i) => i.size_hint(),
            IntoIterInner::String(ref i) => i.size_hint(),
            IntoIterInner::List(ref i) => i.size_hint(),
            IntoIterInner::Compound(ref i) => i.size_hint(),
            IntoIterInner::IntArray(ref i) => i.size_hint(),
            IntoIterInner::LongArray(ref i) => i.size_hint(),
        }
    }
}

impl<S> DoubleEndedIterator for IntoIter<S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.inner {
            IntoIterInner::End => None,
            IntoIterInner::Byte(ref mut i) => i.next_back().map(Value::Byte),
            IntoIterInner::Short(ref mut i) => i.next_back().map(Value::Short),
            IntoIterInner::Int(ref mut i) => i.next_back().map(Value::Int),
            IntoIterInner::Long(ref mut i) => i.next_back().map(Value::Long),
            IntoIterInner::Float(ref mut i) => i.next_back().map(Value::Float),
            IntoIterInner::Double(ref mut i) => i.next_back().map(Value::Double),
            IntoIterInner::ByteArray(ref mut i) => i.next_back().map(Value::ByteArray),
            IntoIterInner::String(ref mut i) => i.next_back().map(Value::String),
            IntoIterInner::List(ref mut i) => i.next_back().map(Value::List),
            IntoIterInner::Compound(ref mut i) => i.next_back().map(Value::Compound),
            IntoIterInner::IntArray(ref mut i) => i.next_back().map(Value::IntArray),
            IntoIterInner::LongArray(ref mut i) => i.next_back().map(Value::LongArray),
        }
    }
}

impl<S> ExactSizeIterator for IntoIter<S> {
    fn len(&self) -> usize {
        match self.inner {
            IntoIterInner::End => 0,
            IntoIterInner::Byte(ref i) => i.len(),
            IntoIterInner::Short(ref i) => i.len(),
            IntoIterInner::Int(ref i) => i.len(),
            IntoIterInner::Long(ref i) => i.len(),
            IntoIterInner::Float(ref i) => i.len(),
            IntoIterInner::Double(ref i) => i.len(),
            IntoIterInner::ByteArray(ref i) => i.len(),
            IntoIterInner::String(ref i) => i.len(),
            IntoIterInner::List(ref i) => i.len(),
            IntoIterInner::Compound(ref i) => i.len(),
            IntoIterInner::IntArray(ref i) => i.len(),
            IntoIterInner::LongArray(ref i) => i.len(),
        }
    }
}

impl<S> FusedIterator for IntoIter<S> {}

/// The borrowing iterator type for [`List`].
pub struct Iter<'a, S = String> {
    inner: IterInner<'a, S>,
}

enum IterInner<'a, S> {
    End,
    Byte(std::slice::Iter<'a, i8>),
    Short(std::slice::Iter<'a, i16>),
    Int(std::slice::Iter<'a, i32>),
    Long(std::slice::Iter<'a, i64>),
    Float(std::slice::Iter<'a, f32>),
    Double(std::slice::Iter<'a, f64>),
    ByteArray(std::slice::Iter<'a, Vec<i8>>),
    String(std::slice::Iter<'a, S>),
    List(std::slice::Iter<'a, List<S>>),
    Compound(std::slice::Iter<'a, Compound<S>>),
    IntArray(std::slice::Iter<'a, Vec<i32>>),
    LongArray(std::slice::Iter<'a, Vec<i64>>),
}

impl<'a, S> Iterator for Iter<'a, S> {
    type Item = ValueRef<'a, S>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            IterInner::End => None,
            IterInner::Byte(ref mut i) => i.next().map(ValueRef::Byte),
            IterInner::Short(ref mut i) => i.next().map(ValueRef::Short),
            IterInner::Int(ref mut i) => i.next().map(ValueRef::Int),
            IterInner::Long(ref mut i) => i.next().map(ValueRef::Long),
            IterInner::Float(ref mut i) => i.next().map(ValueRef::Float),
            IterInner::Double(ref mut i) => i.next().map(ValueRef::Double),
            IterInner::ByteArray(ref mut i) => i.next().map(|arr| ValueRef::ByteArray(&arr[..])),
            IterInner::String(ref mut i) => i.next().map(ValueRef::String),
            IterInner::List(ref mut i) => i.next().map(ValueRef::List),
            IterInner::Compound(ref mut i) => i.next().map(ValueRef::Compound),
            IterInner::IntArray(ref mut i) => i.next().map(|arr| ValueRef::IntArray(&arr[..])),
            IterInner::LongArray(ref mut i) => i.next().map(|arr| ValueRef::LongArray(&arr[..])),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.inner {
            IterInner::End => (0, Some(0)),
            IterInner::Byte(ref i) => i.size_hint(),
            IterInner::Short(ref i) => i.size_hint(),
            IterInner::Int(ref i) => i.size_hint(),
            IterInner::Long(ref i) => i.size_hint(),
            IterInner::Float(ref i) => i.size_hint(),
            IterInner::Double(ref i) => i.size_hint(),
            IterInner::ByteArray(ref i) => i.size_hint(),
            IterInner::String(ref i) => i.size_hint(),
            IterInner::List(ref i) => i.size_hint(),
            IterInner::Compound(ref i) => i.size_hint(),
            IterInner::IntArray(ref i) => i.size_hint(),
            IterInner::LongArray(ref i) => i.size_hint(),
        }
    }
}

impl<S> DoubleEndedIterator for Iter<'_, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.inner {
            IterInner::End => None,
            IterInner::Byte(ref mut i) => i.next_back().map(ValueRef::Byte),
            IterInner::Short(ref mut i) => i.next_back().map(ValueRef::Short),
            IterInner::Int(ref mut i) => i.next_back().map(ValueRef::Int),
            IterInner::Long(ref mut i) => i.next_back().map(ValueRef::Long),
            IterInner::Float(ref mut i) => i.next_back().map(ValueRef::Float),
            IterInner::Double(ref mut i) => i.next_back().map(ValueRef::Double),
            IterInner::ByteArray(ref mut i) => {
                i.next_back().map(|arr| ValueRef::ByteArray(&arr[..]))
            }
            IterInner::String(ref mut i) => i.next_back().map(ValueRef::String),
            IterInner::List(ref mut i) => i.next_back().map(ValueRef::List),
            IterInner::Compound(ref mut i) => i.next_back().map(ValueRef::Compound),
            IterInner::IntArray(ref mut i) => i.next_back().map(|arr| ValueRef::IntArray(&arr[..])),
            IterInner::LongArray(ref mut i) => {
                i.next_back().map(|arr| ValueRef::LongArray(&arr[..]))
            }
        }
    }
}

impl<S> ExactSizeIterator for Iter<'_, S> {
    fn len(&self) -> usize {
        match self.inner {
            IterInner::End => 0,
            IterInner::Byte(ref i) => i.len(),
            IterInner::Short(ref i) => i.len(),
            IterInner::Int(ref i) => i.len(),
            IterInner::Long(ref i) => i.len(),
            IterInner::Float(ref i) => i.len(),
            IterInner::Double(ref i) => i.len(),
            IterInner::ByteArray(ref i) => i.len(),
            IterInner::String(ref i) => i.len(),
            IterInner::List(ref i) => i.len(),
            IterInner::Compound(ref i) => i.len(),
            IterInner::IntArray(ref i) => i.len(),
            IterInner::LongArray(ref i) => i.len(),
        }
    }
}

impl<S> FusedIterator for Iter<'_, S> {}

/// The mutable borrowing iterator type for [`List`].
pub struct IterMut<'a, S = String> {
    inner: IterMutInner<'a, S>,
}

enum IterMutInner<'a, S> {
    End,
    Byte(std::slice::IterMut<'a, i8>),
    Short(std::slice::IterMut<'a, i16>),
    Int(std::slice::IterMut<'a, i32>),
    Long(std::slice::IterMut<'a, i64>),
    Float(std::slice::IterMut<'a, f32>),
    Double(std::slice::IterMut<'a, f64>),
    ByteArray(std::slice::IterMut<'a, Vec<i8>>),
    String(std::slice::IterMut<'a, S>),
    List(std::slice::IterMut<'a, List<S>>),
    Compound(std::slice::IterMut<'a, Compound<S>>),
    IntArray(std::slice::IterMut<'a, Vec<i32>>),
    LongArray(std::slice::IterMut<'a, Vec<i64>>),
}

impl<'a, S> Iterator for IterMut<'a, S> {
    type Item = ValueMut<'a, S>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            IterMutInner::End => None,
            IterMutInner::Byte(ref mut i) => i.next().map(ValueMut::Byte),
            IterMutInner::Short(ref mut i) => i.next().map(ValueMut::Short),
            IterMutInner::Int(ref mut i) => i.next().map(ValueMut::Int),
            IterMutInner::Long(ref mut i) => i.next().map(ValueMut::Long),
            IterMutInner::Float(ref mut i) => i.next().map(ValueMut::Float),
            IterMutInner::Double(ref mut i) => i.next().map(ValueMut::Double),
            IterMutInner::ByteArray(ref mut i) => i.next().map(ValueMut::ByteArray),
            IterMutInner::String(ref mut i) => i.next().map(ValueMut::String),
            IterMutInner::List(ref mut i) => i.next().map(ValueMut::List),
            IterMutInner::Compound(ref mut i) => i.next().map(ValueMut::Compound),
            IterMutInner::IntArray(ref mut i) => i.next().map(ValueMut::IntArray),
            IterMutInner::LongArray(ref mut i) => i.next().map(ValueMut::LongArray),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.inner {
            IterMutInner::End => (0, Some(0)),
            IterMutInner::Byte(ref i) => i.size_hint(),
            IterMutInner::Short(ref i) => i.size_hint(),
            IterMutInner::Int(ref i) => i.size_hint(),
            IterMutInner::Long(ref i) => i.size_hint(),
            IterMutInner::Float(ref i) => i.size_hint(),
            IterMutInner::Double(ref i) => i.size_hint(),
            IterMutInner::ByteArray(ref i) => i.size_hint(),
            IterMutInner::String(ref i) => i.size_hint(),
            IterMutInner::List(ref i) => i.size_hint(),
            IterMutInner::Compound(ref i) => i.size_hint(),
            IterMutInner::IntArray(ref i) => i.size_hint(),
            IterMutInner::LongArray(ref i) => i.size_hint(),
        }
    }
}

impl<S> DoubleEndedIterator for IterMut<'_, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.inner {
            IterMutInner::End => None,
            IterMutInner::Byte(ref mut i) => i.next_back().map(ValueMut::Byte),
            IterMutInner::Short(ref mut i) => i.next_back().map(ValueMut::Short),
            IterMutInner::Int(ref mut i) => i.next_back().map(ValueMut::Int),
            IterMutInner::Long(ref mut i) => i.next_back().map(ValueMut::Long),
            IterMutInner::Float(ref mut i) => i.next_back().map(ValueMut::Float),
            IterMutInner::Double(ref mut i) => i.next_back().map(ValueMut::Double),
            IterMutInner::ByteArray(ref mut i) => i.next_back().map(ValueMut::ByteArray),
            IterMutInner::String(ref mut i) => i.next_back().map(ValueMut::String),
            IterMutInner::List(ref mut i) => i.next_back().map(ValueMut::List),
            IterMutInner::Compound(ref mut i) => i.next_back().map(ValueMut::Compound),
            IterMutInner::IntArray(ref mut i) => i.next_back().map(ValueMut::IntArray),
            IterMutInner::LongArray(ref mut i) => i.next_back().map(ValueMut::LongArray),
        }
    }
}

impl<S> ExactSizeIterator for IterMut<'_, S> {
    fn len(&self) -> usize {
        match self.inner {
            IterMutInner::End => 0,
            IterMutInner::Byte(ref i) => i.len(),
            IterMutInner::Short(ref i) => i.len(),
            IterMutInner::Int(ref i) => i.len(),
            IterMutInner::Long(ref i) => i.len(),
            IterMutInner::Float(ref i) => i.len(),
            IterMutInner::Double(ref i) => i.len(),
            IterMutInner::ByteArray(ref i) => i.len(),
            IterMutInner::String(ref i) => i.len(),
            IterMutInner::List(ref i) => i.len(),
            IterMutInner::Compound(ref i) => i.len(),
            IterMutInner::IntArray(ref i) => i.len(),
            IterMutInner::LongArray(ref i) => i.len(),
        }
    }
}

impl<S> FusedIterator for IterMut<'_, S> {}
