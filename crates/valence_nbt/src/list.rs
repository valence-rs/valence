use std::iter::FusedIterator;

use crate::value::{ValueRef, ValueRefMut};
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
#[derive(Clone, Default, PartialEq, Debug)]
pub enum List {
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
    String(Vec<String>),
    List(Vec<List>),
    Compound(Vec<Compound>),
    IntArray(Vec<Vec<i32>>),
    LongArray(Vec<Vec<i64>>),
}

impl List {
    /// Constructs a new empty NBT list, with the element type of `TAG_End`.
    pub fn new() -> List {
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
    pub fn get(&self, index: usize) -> Option<ValueRef> {
        match self {
            List::End => None,
            List::Byte(list) => list.get(index).map(ValueRef::Byte),
            List::Short(list) => list.get(index).map(ValueRef::Short),
            List::Int(list) => list.get(index).map(ValueRef::Int),
            List::Long(list) => list.get(index).map(ValueRef::Long),
            List::Float(list) => list.get(index).map(ValueRef::Float),
            List::Double(list) => list.get(index).map(ValueRef::Double),
            List::ByteArray(list) => list.get(index).map(|arr| ValueRef::ByteArray(&arr[..])),
            List::String(list) => list.get(index).map(|str| ValueRef::String(&str[..])),
            List::List(list) => list.get(index).map(ValueRef::List),
            List::Compound(list) => list.get(index).map(ValueRef::Compound),
            List::IntArray(list) => list.get(index).map(|arr| ValueRef::IntArray(&arr[..])),
            List::LongArray(list) => list.get(index).map(|arr| ValueRef::LongArray(&arr[..])),
        }
    }

    /// Gets a mutable reference to the value at the given index in this list,
    /// or `None` if the index is out of bounds.
    pub fn get_mut(&mut self, index: usize) -> Option<ValueRefMut> {
        match self {
            List::End => None,
            List::Byte(list) => list.get_mut(index).map(ValueRefMut::Byte),
            List::Short(list) => list.get_mut(index).map(ValueRefMut::Short),
            List::Int(list) => list.get_mut(index).map(ValueRefMut::Int),
            List::Long(list) => list.get_mut(index).map(ValueRefMut::Long),
            List::Float(list) => list.get_mut(index).map(ValueRefMut::Float),
            List::Double(list) => list.get_mut(index).map(ValueRefMut::Double),
            List::ByteArray(list) => list.get_mut(index).map(ValueRefMut::ByteArray),
            List::String(list) => list.get_mut(index).map(ValueRefMut::String),
            List::List(list) => list.get_mut(index).map(ValueRefMut::List),
            List::Compound(list) => list.get_mut(index).map(ValueRefMut::Compound),
            List::IntArray(list) => list.get_mut(index).map(ValueRefMut::IntArray),
            List::LongArray(list) => list.get_mut(index).map(ValueRefMut::LongArray),
        }
    }

    /// Attempts to add the given value to the end of this list, failing if
    /// adding the value would result in the list not being heterogeneous (have
    /// multiple types inside it). Returns `true` if the value was added,
    /// `false` otherwise.
    #[must_use]
    pub fn try_push(&mut self, value: impl Into<Value>) -> bool {
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
    pub fn try_insert(&mut self, index: usize, value: impl Into<Value>) -> bool {
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
    pub fn remove(&mut self, index: usize) -> Value {
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn assert_failed(index: usize, len: usize) -> ! {
            panic!("removal index (is {index}) should be < len (is {len})");
        }

        match self {
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
        }
    }

    /// Returns an iterator over this list. This iterator yields [ValueRef]s.
    pub fn iter(&self) -> ListIter {
        ListIter {
            inner: match self {
                List::End => ListIterInner::End,
                List::Byte(list) => ListIterInner::Byte(list.iter()),
                List::Short(list) => ListIterInner::Short(list.iter()),
                List::Int(list) => ListIterInner::Int(list.iter()),
                List::Long(list) => ListIterInner::Long(list.iter()),
                List::Float(list) => ListIterInner::Float(list.iter()),
                List::Double(list) => ListIterInner::Double(list.iter()),
                List::ByteArray(list) => ListIterInner::ByteArray(list.iter()),
                List::String(list) => ListIterInner::String(list.iter()),
                List::List(list) => ListIterInner::List(list.iter()),
                List::Compound(list) => ListIterInner::Compound(list.iter()),
                List::IntArray(list) => ListIterInner::IntArray(list.iter()),
                List::LongArray(list) => ListIterInner::LongArray(list.iter()),
            },
        }
    }

    /// Returns a mutable iterator over this list. This iterator yields
    /// [ValueRefMut]s.
    pub fn iter_mut(&mut self) -> ListIterMut {
        ListIterMut {
            inner: match self {
                List::End => ListIterMutInner::End,
                List::Byte(list) => ListIterMutInner::Byte(list.iter_mut()),
                List::Short(list) => ListIterMutInner::Short(list.iter_mut()),
                List::Int(list) => ListIterMutInner::Int(list.iter_mut()),
                List::Long(list) => ListIterMutInner::Long(list.iter_mut()),
                List::Float(list) => ListIterMutInner::Float(list.iter_mut()),
                List::Double(list) => ListIterMutInner::Double(list.iter_mut()),
                List::ByteArray(list) => ListIterMutInner::ByteArray(list.iter_mut()),
                List::String(list) => ListIterMutInner::String(list.iter_mut()),
                List::List(list) => ListIterMutInner::List(list.iter_mut()),
                List::Compound(list) => ListIterMutInner::Compound(list.iter_mut()),
                List::IntArray(list) => ListIterMutInner::IntArray(list.iter_mut()),
                List::LongArray(list) => ListIterMutInner::LongArray(list.iter_mut()),
            },
        }
    }
}

impl From<Vec<i8>> for List {
    fn from(v: Vec<i8>) -> Self {
        List::Byte(v)
    }
}

impl From<Vec<i16>> for List {
    fn from(v: Vec<i16>) -> Self {
        List::Short(v)
    }
}

impl From<Vec<i32>> for List {
    fn from(v: Vec<i32>) -> Self {
        List::Int(v)
    }
}

impl From<Vec<i64>> for List {
    fn from(v: Vec<i64>) -> Self {
        List::Long(v)
    }
}

impl From<Vec<f32>> for List {
    fn from(v: Vec<f32>) -> Self {
        List::Float(v)
    }
}

impl From<Vec<f64>> for List {
    fn from(v: Vec<f64>) -> Self {
        List::Double(v)
    }
}

impl From<Vec<Vec<i8>>> for List {
    fn from(v: Vec<Vec<i8>>) -> Self {
        List::ByteArray(v)
    }
}

impl From<Vec<String>> for List {
    fn from(v: Vec<String>) -> Self {
        List::String(v)
    }
}

impl From<Vec<List>> for List {
    fn from(v: Vec<List>) -> Self {
        List::List(v)
    }
}

impl From<Vec<Compound>> for List {
    fn from(v: Vec<Compound>) -> Self {
        List::Compound(v)
    }
}

impl From<Vec<Vec<i32>>> for List {
    fn from(v: Vec<Vec<i32>>) -> Self {
        List::IntArray(v)
    }
}

impl From<Vec<Vec<i64>>> for List {
    fn from(v: Vec<Vec<i64>>) -> Self {
        List::LongArray(v)
    }
}

/// Converts a value to a singleton list.
impl From<Value> for List {
    fn from(value: Value) -> Self {
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

impl IntoIterator for List {
    type Item = Value;
    type IntoIter = ListIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        ListIntoIter {
            inner: match self {
                List::End => ListIntoIterInner::End,
                List::Byte(list) => ListIntoIterInner::Byte(list.into_iter()),
                List::Short(list) => ListIntoIterInner::Short(list.into_iter()),
                List::Int(list) => ListIntoIterInner::Int(list.into_iter()),
                List::Long(list) => ListIntoIterInner::Long(list.into_iter()),
                List::Float(list) => ListIntoIterInner::Float(list.into_iter()),
                List::Double(list) => ListIntoIterInner::Double(list.into_iter()),
                List::ByteArray(list) => ListIntoIterInner::ByteArray(list.into_iter()),
                List::String(list) => ListIntoIterInner::String(list.into_iter()),
                List::List(list) => ListIntoIterInner::List(list.into_iter()),
                List::Compound(list) => ListIntoIterInner::Compound(list.into_iter()),
                List::IntArray(list) => ListIntoIterInner::IntArray(list.into_iter()),
                List::LongArray(list) => ListIntoIterInner::LongArray(list.into_iter()),
            },
        }
    }
}

impl<'a> IntoIterator for &'a List {
    type Item = ValueRef<'a>;
    type IntoIter = ListIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut List {
    type Item = ValueRefMut<'a>;
    type IntoIter = ListIterMut<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// The owned iterator type for [List].
pub struct ListIntoIter {
    inner: ListIntoIterInner,
}

enum ListIntoIterInner {
    End,
    Byte(std::vec::IntoIter<i8>),
    Short(std::vec::IntoIter<i16>),
    Int(std::vec::IntoIter<i32>),
    Long(std::vec::IntoIter<i64>),
    Float(std::vec::IntoIter<f32>),
    Double(std::vec::IntoIter<f64>),
    ByteArray(std::vec::IntoIter<Vec<i8>>),
    String(std::vec::IntoIter<String>),
    List(std::vec::IntoIter<List>),
    Compound(std::vec::IntoIter<Compound>),
    IntArray(std::vec::IntoIter<Vec<i32>>),
    LongArray(std::vec::IntoIter<Vec<i64>>),
}

impl Iterator for ListIntoIter {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            ListIntoIterInner::End => None,
            ListIntoIterInner::Byte(ref mut i) => i.next().map(Value::Byte),
            ListIntoIterInner::Short(ref mut i) => i.next().map(Value::Short),
            ListIntoIterInner::Int(ref mut i) => i.next().map(Value::Int),
            ListIntoIterInner::Long(ref mut i) => i.next().map(Value::Long),
            ListIntoIterInner::Float(ref mut i) => i.next().map(Value::Float),
            ListIntoIterInner::Double(ref mut i) => i.next().map(Value::Double),
            ListIntoIterInner::ByteArray(ref mut i) => i.next().map(Value::ByteArray),
            ListIntoIterInner::String(ref mut i) => i.next().map(Value::String),
            ListIntoIterInner::List(ref mut i) => i.next().map(Value::List),
            ListIntoIterInner::Compound(ref mut i) => i.next().map(Value::Compound),
            ListIntoIterInner::IntArray(ref mut i) => i.next().map(Value::IntArray),
            ListIntoIterInner::LongArray(ref mut i) => i.next().map(Value::LongArray),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.inner {
            ListIntoIterInner::End => (0, Some(0)),
            ListIntoIterInner::Byte(ref i) => i.size_hint(),
            ListIntoIterInner::Short(ref i) => i.size_hint(),
            ListIntoIterInner::Int(ref i) => i.size_hint(),
            ListIntoIterInner::Long(ref i) => i.size_hint(),
            ListIntoIterInner::Float(ref i) => i.size_hint(),
            ListIntoIterInner::Double(ref i) => i.size_hint(),
            ListIntoIterInner::ByteArray(ref i) => i.size_hint(),
            ListIntoIterInner::String(ref i) => i.size_hint(),
            ListIntoIterInner::List(ref i) => i.size_hint(),
            ListIntoIterInner::Compound(ref i) => i.size_hint(),
            ListIntoIterInner::IntArray(ref i) => i.size_hint(),
            ListIntoIterInner::LongArray(ref i) => i.size_hint(),
        }
    }
}

impl DoubleEndedIterator for ListIntoIter {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.inner {
            ListIntoIterInner::End => None,
            ListIntoIterInner::Byte(ref mut i) => i.next_back().map(Value::Byte),
            ListIntoIterInner::Short(ref mut i) => i.next_back().map(Value::Short),
            ListIntoIterInner::Int(ref mut i) => i.next_back().map(Value::Int),
            ListIntoIterInner::Long(ref mut i) => i.next_back().map(Value::Long),
            ListIntoIterInner::Float(ref mut i) => i.next_back().map(Value::Float),
            ListIntoIterInner::Double(ref mut i) => i.next_back().map(Value::Double),
            ListIntoIterInner::ByteArray(ref mut i) => i.next_back().map(Value::ByteArray),
            ListIntoIterInner::String(ref mut i) => i.next_back().map(Value::String),
            ListIntoIterInner::List(ref mut i) => i.next_back().map(Value::List),
            ListIntoIterInner::Compound(ref mut i) => i.next_back().map(Value::Compound),
            ListIntoIterInner::IntArray(ref mut i) => i.next_back().map(Value::IntArray),
            ListIntoIterInner::LongArray(ref mut i) => i.next_back().map(Value::LongArray),
        }
    }
}

impl ExactSizeIterator for ListIntoIter {
    fn len(&self) -> usize {
        match self.inner {
            ListIntoIterInner::End => 0,
            ListIntoIterInner::Byte(ref i) => i.len(),
            ListIntoIterInner::Short(ref i) => i.len(),
            ListIntoIterInner::Int(ref i) => i.len(),
            ListIntoIterInner::Long(ref i) => i.len(),
            ListIntoIterInner::Float(ref i) => i.len(),
            ListIntoIterInner::Double(ref i) => i.len(),
            ListIntoIterInner::ByteArray(ref i) => i.len(),
            ListIntoIterInner::String(ref i) => i.len(),
            ListIntoIterInner::List(ref i) => i.len(),
            ListIntoIterInner::Compound(ref i) => i.len(),
            ListIntoIterInner::IntArray(ref i) => i.len(),
            ListIntoIterInner::LongArray(ref i) => i.len(),
        }
    }
}

impl FusedIterator for ListIntoIter {}

/// The borrowing iterator type for [List].
pub struct ListIter<'a> {
    inner: ListIterInner<'a>,
}

enum ListIterInner<'a> {
    End,
    Byte(std::slice::Iter<'a, i8>),
    Short(std::slice::Iter<'a, i16>),
    Int(std::slice::Iter<'a, i32>),
    Long(std::slice::Iter<'a, i64>),
    Float(std::slice::Iter<'a, f32>),
    Double(std::slice::Iter<'a, f64>),
    ByteArray(std::slice::Iter<'a, Vec<i8>>),
    String(std::slice::Iter<'a, String>),
    List(std::slice::Iter<'a, List>),
    Compound(std::slice::Iter<'a, Compound>),
    IntArray(std::slice::Iter<'a, Vec<i32>>),
    LongArray(std::slice::Iter<'a, Vec<i64>>),
}

impl<'a> Iterator for ListIter<'a> {
    type Item = ValueRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            ListIterInner::End => None,
            ListIterInner::Byte(ref mut i) => i.next().map(ValueRef::Byte),
            ListIterInner::Short(ref mut i) => i.next().map(ValueRef::Short),
            ListIterInner::Int(ref mut i) => i.next().map(ValueRef::Int),
            ListIterInner::Long(ref mut i) => i.next().map(ValueRef::Long),
            ListIterInner::Float(ref mut i) => i.next().map(ValueRef::Float),
            ListIterInner::Double(ref mut i) => i.next().map(ValueRef::Double),
            ListIterInner::ByteArray(ref mut i) => {
                i.next().map(|arr| ValueRef::ByteArray(&arr[..]))
            }
            ListIterInner::String(ref mut i) => i.next().map(|str| ValueRef::String(&str[..])),
            ListIterInner::List(ref mut i) => i.next().map(ValueRef::List),
            ListIterInner::Compound(ref mut i) => i.next().map(ValueRef::Compound),
            ListIterInner::IntArray(ref mut i) => i.next().map(|arr| ValueRef::IntArray(&arr[..])),
            ListIterInner::LongArray(ref mut i) => {
                i.next().map(|arr| ValueRef::LongArray(&arr[..]))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.inner {
            ListIterInner::End => (0, Some(0)),
            ListIterInner::Byte(ref i) => i.size_hint(),
            ListIterInner::Short(ref i) => i.size_hint(),
            ListIterInner::Int(ref i) => i.size_hint(),
            ListIterInner::Long(ref i) => i.size_hint(),
            ListIterInner::Float(ref i) => i.size_hint(),
            ListIterInner::Double(ref i) => i.size_hint(),
            ListIterInner::ByteArray(ref i) => i.size_hint(),
            ListIterInner::String(ref i) => i.size_hint(),
            ListIterInner::List(ref i) => i.size_hint(),
            ListIterInner::Compound(ref i) => i.size_hint(),
            ListIterInner::IntArray(ref i) => i.size_hint(),
            ListIterInner::LongArray(ref i) => i.size_hint(),
        }
    }
}

impl DoubleEndedIterator for ListIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.inner {
            ListIterInner::End => None,
            ListIterInner::Byte(ref mut i) => i.next_back().map(ValueRef::Byte),
            ListIterInner::Short(ref mut i) => i.next_back().map(ValueRef::Short),
            ListIterInner::Int(ref mut i) => i.next_back().map(ValueRef::Int),
            ListIterInner::Long(ref mut i) => i.next_back().map(ValueRef::Long),
            ListIterInner::Float(ref mut i) => i.next_back().map(ValueRef::Float),
            ListIterInner::Double(ref mut i) => i.next_back().map(ValueRef::Double),
            ListIterInner::ByteArray(ref mut i) => {
                i.next_back().map(|arr| ValueRef::ByteArray(&arr[..]))
            }
            ListIterInner::String(ref mut i) => i.next_back().map(|str| ValueRef::String(&str[..])),
            ListIterInner::List(ref mut i) => i.next_back().map(ValueRef::List),
            ListIterInner::Compound(ref mut i) => i.next_back().map(ValueRef::Compound),
            ListIterInner::IntArray(ref mut i) => {
                i.next_back().map(|arr| ValueRef::IntArray(&arr[..]))
            }
            ListIterInner::LongArray(ref mut i) => {
                i.next_back().map(|arr| ValueRef::LongArray(&arr[..]))
            }
        }
    }
}

impl ExactSizeIterator for ListIter<'_> {
    fn len(&self) -> usize {
        match self.inner {
            ListIterInner::End => 0,
            ListIterInner::Byte(ref i) => i.len(),
            ListIterInner::Short(ref i) => i.len(),
            ListIterInner::Int(ref i) => i.len(),
            ListIterInner::Long(ref i) => i.len(),
            ListIterInner::Float(ref i) => i.len(),
            ListIterInner::Double(ref i) => i.len(),
            ListIterInner::ByteArray(ref i) => i.len(),
            ListIterInner::String(ref i) => i.len(),
            ListIterInner::List(ref i) => i.len(),
            ListIterInner::Compound(ref i) => i.len(),
            ListIterInner::IntArray(ref i) => i.len(),
            ListIterInner::LongArray(ref i) => i.len(),
        }
    }
}

impl FusedIterator for ListIter<'_> {}

/// The mutable borrowing iterator type for [List].
pub struct ListIterMut<'a> {
    inner: ListIterMutInner<'a>,
}

enum ListIterMutInner<'a> {
    End,
    Byte(std::slice::IterMut<'a, i8>),
    Short(std::slice::IterMut<'a, i16>),
    Int(std::slice::IterMut<'a, i32>),
    Long(std::slice::IterMut<'a, i64>),
    Float(std::slice::IterMut<'a, f32>),
    Double(std::slice::IterMut<'a, f64>),
    ByteArray(std::slice::IterMut<'a, Vec<i8>>),
    String(std::slice::IterMut<'a, String>),
    List(std::slice::IterMut<'a, List>),
    Compound(std::slice::IterMut<'a, Compound>),
    IntArray(std::slice::IterMut<'a, Vec<i32>>),
    LongArray(std::slice::IterMut<'a, Vec<i64>>),
}

impl<'a> Iterator for ListIterMut<'a> {
    type Item = ValueRefMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            ListIterMutInner::End => None,
            ListIterMutInner::Byte(ref mut i) => i.next().map(ValueRefMut::Byte),
            ListIterMutInner::Short(ref mut i) => i.next().map(ValueRefMut::Short),
            ListIterMutInner::Int(ref mut i) => i.next().map(ValueRefMut::Int),
            ListIterMutInner::Long(ref mut i) => i.next().map(ValueRefMut::Long),
            ListIterMutInner::Float(ref mut i) => i.next().map(ValueRefMut::Float),
            ListIterMutInner::Double(ref mut i) => i.next().map(ValueRefMut::Double),
            ListIterMutInner::ByteArray(ref mut i) => i.next().map(ValueRefMut::ByteArray),
            ListIterMutInner::String(ref mut i) => i.next().map(ValueRefMut::String),
            ListIterMutInner::List(ref mut i) => i.next().map(ValueRefMut::List),
            ListIterMutInner::Compound(ref mut i) => i.next().map(ValueRefMut::Compound),
            ListIterMutInner::IntArray(ref mut i) => i.next().map(ValueRefMut::IntArray),
            ListIterMutInner::LongArray(ref mut i) => i.next().map(ValueRefMut::LongArray),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.inner {
            ListIterMutInner::End => (0, Some(0)),
            ListIterMutInner::Byte(ref i) => i.size_hint(),
            ListIterMutInner::Short(ref i) => i.size_hint(),
            ListIterMutInner::Int(ref i) => i.size_hint(),
            ListIterMutInner::Long(ref i) => i.size_hint(),
            ListIterMutInner::Float(ref i) => i.size_hint(),
            ListIterMutInner::Double(ref i) => i.size_hint(),
            ListIterMutInner::ByteArray(ref i) => i.size_hint(),
            ListIterMutInner::String(ref i) => i.size_hint(),
            ListIterMutInner::List(ref i) => i.size_hint(),
            ListIterMutInner::Compound(ref i) => i.size_hint(),
            ListIterMutInner::IntArray(ref i) => i.size_hint(),
            ListIterMutInner::LongArray(ref i) => i.size_hint(),
        }
    }
}

impl DoubleEndedIterator for ListIterMut<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.inner {
            ListIterMutInner::End => None,
            ListIterMutInner::Byte(ref mut i) => i.next_back().map(ValueRefMut::Byte),
            ListIterMutInner::Short(ref mut i) => i.next_back().map(ValueRefMut::Short),
            ListIterMutInner::Int(ref mut i) => i.next_back().map(ValueRefMut::Int),
            ListIterMutInner::Long(ref mut i) => i.next_back().map(ValueRefMut::Long),
            ListIterMutInner::Float(ref mut i) => i.next_back().map(ValueRefMut::Float),
            ListIterMutInner::Double(ref mut i) => i.next_back().map(ValueRefMut::Double),
            ListIterMutInner::ByteArray(ref mut i) => i.next_back().map(ValueRefMut::ByteArray),
            ListIterMutInner::String(ref mut i) => i.next_back().map(ValueRefMut::String),
            ListIterMutInner::List(ref mut i) => i.next_back().map(ValueRefMut::List),
            ListIterMutInner::Compound(ref mut i) => i.next_back().map(ValueRefMut::Compound),
            ListIterMutInner::IntArray(ref mut i) => i.next_back().map(ValueRefMut::IntArray),
            ListIterMutInner::LongArray(ref mut i) => i.next_back().map(ValueRefMut::LongArray),
        }
    }
}

impl ExactSizeIterator for ListIterMut<'_> {
    fn len(&self) -> usize {
        match self.inner {
            ListIterMutInner::End => 0,
            ListIterMutInner::Byte(ref i) => i.len(),
            ListIterMutInner::Short(ref i) => i.len(),
            ListIterMutInner::Int(ref i) => i.len(),
            ListIterMutInner::Long(ref i) => i.len(),
            ListIterMutInner::Float(ref i) => i.len(),
            ListIterMutInner::Double(ref i) => i.len(),
            ListIterMutInner::ByteArray(ref i) => i.len(),
            ListIterMutInner::String(ref i) => i.len(),
            ListIterMutInner::List(ref i) => i.len(),
            ListIterMutInner::Compound(ref i) => i.len(),
            ListIterMutInner::IntArray(ref i) => i.len(),
            ListIterMutInner::LongArray(ref i) => i.len(),
        }
    }
}

impl FusedIterator for ListIterMut<'_> {}
