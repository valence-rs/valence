#![allow(dead_code)]

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::iter::FusedIterator;
use std::sync::Arc;

use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::slab::Slab;

#[derive(Clone, Debug)]
pub struct SlabRc<T> {
    slab: Slab<Slot<T>>,
}

#[derive(Debug)]
struct Slot<T> {
    value: T,
    key: Key,
}

impl<T: Clone> Clone for Slot<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            key: Key(Arc::new(*self.key.0)),
        }
    }
}

#[derive(Clone, Eq, Debug)]
pub struct Key(Arc<usize>);

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        Arc::as_ptr(&self.0) == Arc::as_ptr(&other.0)
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Arc::as_ptr(&self.0).partial_cmp(&Arc::as_ptr(&other.0))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state)
    }
}

impl<T> SlabRc<T> {
    pub const fn new() -> Self {
        Self { slab: Slab::new() }
    }

    pub fn get(&self, key: &Key) -> &T {
        let slot = self.slab.get(*key.0).expect("invalid key");
        debug_assert_eq!(&slot.key, key, "invalid key");

        &slot.value
    }

    pub fn get_mut(&mut self, key: &Key) -> &mut T {
        let slot = self.slab.get_mut(*key.0).expect("invalid key");
        debug_assert_eq!(&slot.key, key, "invalid key");

        &mut slot.value
    }

    pub fn len(&self) -> usize {
        self.slab.len()
    }

    pub fn insert(&mut self, value: T) -> (Key, &mut T) {
        let (_, slot) = self.slab.insert_with(|idx| Slot {
            value,
            key: Key(Arc::new(idx)),
        });

        (slot.key.clone(), &mut slot.value)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (&Key, &T)> + FusedIterator + Clone + '_ {
        self.slab.iter().map(|(_, slot)| (&slot.key, &slot.value))
    }

    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (&Key, &mut T)> + FusedIterator + '_ {
        self.slab
            .iter_mut()
            .map(|(_, slot)| (&slot.key, &mut slot.value))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (&Key, &T)> + Clone + '_
    where
        T: Sync,
    {
        self.slab
            .par_iter()
            .map(|(_, slot)| (&slot.key, &slot.value))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (&Key, &mut T)> + '_
    where
        T: Send + Sync,
    {
        self.slab
            .par_iter_mut()
            .map(|(_, slot)| (&slot.key, &mut slot.value))
    }

    pub fn collect_garbage(&mut self) {
        self.slab
            .retain(|_, slot| (Arc::strong_count(&slot.key.0) > 1).into());
    }
}
