use std::iter::FusedIterator;
use std::{iter, mem, slice};

use rayon::iter::plumbing::UnindexedConsumer;
use rayon::prelude::*;

#[derive(Clone, Debug)]
pub struct Slab<T> {
    entries: Vec<Entry<T>>,
    next_free_head: usize,
    len: usize,
}

impl<T> Default for Slab<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
enum Entry<T> {
    Occupied(T),
    Vacant { next_free: usize },
}

impl<T> Slab<T> {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_free_head: 0,
            len: 0,
        }
    }

    pub fn get(&self, key: usize) -> Option<&T> {
        match self.entries.get(key)? {
            Entry::Occupied(value) => Some(value),
            Entry::Vacant { .. } => None,
        }
    }

    pub fn get_mut(&mut self, key: usize) -> Option<&mut T> {
        match self.entries.get_mut(key)? {
            Entry::Occupied(value) => Some(value),
            Entry::Vacant { .. } => None,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn insert(&mut self, value: T) -> (usize, &mut T) {
        self.insert_with(|_| value)
    }

    pub fn insert_with(&mut self, f: impl FnOnce(usize) -> T) -> (usize, &mut T) {
        self.len += 1;

        if self.next_free_head == self.entries.len() {
            let key = self.next_free_head;

            self.next_free_head += 1;

            self.entries.push(Entry::Occupied(f(key)));

            match self.entries.last_mut() {
                Some(Entry::Occupied(value)) => (key, value),
                _ => unreachable!(),
            }
        } else {
            let entry = &mut self.entries[self.next_free_head];

            let next_free = match entry {
                Entry::Occupied(_) => unreachable!("corrupt free list"),
                Entry::Vacant { next_free } => *next_free,
            };

            let key = self.next_free_head;

            *entry = Entry::Occupied(f(key));

            self.next_free_head = next_free;

            match entry {
                Entry::Occupied(value) => (key, value),
                Entry::Vacant { .. } => unreachable!(),
            }
        }
    }

    pub fn remove(&mut self, key: usize) -> Option<T> {
        let entry = self.entries.get_mut(key)?;
        match entry {
            Entry::Occupied(_) => {
                let old_entry = mem::replace(
                    entry,
                    Entry::Vacant {
                        next_free: self.next_free_head,
                    },
                );

                self.next_free_head = key;
                self.len -= 1;

                match old_entry {
                    Entry::Occupied(value) => Some(value),
                    Entry::Vacant { .. } => unreachable!(),
                }
            }
            Entry::Vacant { .. } => None,
        }
    }

    pub fn retain(&mut self, mut f: impl FnMut(usize, &mut T) -> bool) {
        for (key, entry) in self.entries.iter_mut().enumerate() {
            if let Entry::Occupied(value) = entry {
                if !f(key, value) {
                    *entry = Entry::Vacant {
                        next_free: self.next_free_head,
                    };

                    self.next_free_head = key;
                    self.len -= 1;
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_free_head = 0;
        self.len = 0;
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            entries: self.entries.iter().enumerate(),
            len: self.len,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            entries: self.entries.iter_mut().enumerate(),
            len: self.len,
        }
    }
}

impl<'a, T> IntoIterator for &'a Slab<T> {
    type Item = (usize, &'a T);
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Slab<T> {
    type Item = (usize, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'a, T: Sync> IntoParallelIterator for &'a Slab<T> {
    type Iter = ParIter<'a, T>;
    type Item = (usize, &'a T);

    fn into_par_iter(self) -> Self::Iter {
        ParIter { slab: self }
    }
}

impl<'a, T: Send + Sync> IntoParallelIterator for &'a mut Slab<T> {
    type Iter = ParIterMut<'a, T>;
    type Item = (usize, &'a mut T);

    fn into_par_iter(self) -> Self::Iter {
        ParIterMut { slab: self }
    }
}

pub struct Iter<'a, T> {
    entries: iter::Enumerate<slice::Iter<'a, Entry<T>>>,
    len: usize,
}

pub struct IterMut<'a, T> {
    entries: iter::Enumerate<slice::IterMut<'a, Entry<T>>>,
    len: usize,
}

pub struct ParIter<'a, T> {
    slab: &'a Slab<T>,
}

pub struct ParIterMut<'a, T> {
    slab: &'a mut Slab<T>,
}

impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            len: self.len,
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        for (key, entry) in &mut self.entries {
            if let Entry::Occupied(value) = entry {
                self.len -= 1;
                return Some((key, value));
            }
        }

        debug_assert_eq!(self.len, 0);
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<T> DoubleEndedIterator for Iter<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while let Some((key, entry)) = self.entries.next_back() {
            if let Entry::Occupied(value) = entry {
                self.len -= 1;
                return Some((key, value));
            }
        }

        debug_assert_eq!(self.len, 0);
        None
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<T> FusedIterator for Iter<'_, T> {}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        for (key, entry) in &mut self.entries {
            if let Entry::Occupied(value) = entry {
                self.len -= 1;
                return Some((key, value));
            }
        }

        debug_assert_eq!(self.len, 0);
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<T> DoubleEndedIterator for IterMut<'_, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while let Some((key, entry)) = self.entries.next_back() {
            if let Entry::Occupied(value) = entry {
                self.len -= 1;
                return Some((key, value));
            }
        }

        debug_assert_eq!(self.len, 0);
        None
    }
}

impl<T> ExactSizeIterator for IterMut<'_, T> {
    fn len(&self) -> usize {
        self.len
    }
}

impl<T> FusedIterator for IterMut<'_, T> {}

impl<T> Clone for ParIter<'_, T> {
    fn clone(&self) -> Self {
        Self { slab: &self.slab }
    }
}

impl<'a, T: Sync> ParallelIterator for ParIter<'a, T> {
    type Item = (usize, &'a T);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.slab
            .entries
            .par_iter()
            .enumerate()
            .filter_map(|(key, value)| match value {
                Entry::Occupied(value) => Some((key, value)),
                Entry::Vacant { .. } => None,
            })
            .drive_unindexed(consumer)
    }
}

impl<'a, T: Send + Sync> ParallelIterator for ParIterMut<'a, T> {
    type Item = (usize, &'a mut T);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.slab
            .entries
            .par_iter_mut()
            .enumerate()
            .filter_map(|(key, value)| match value {
                Entry::Occupied(value) => Some((key, value)),
                Entry::Vacant { .. } => None,
            })
            .drive_unindexed(consumer)
    }
}
