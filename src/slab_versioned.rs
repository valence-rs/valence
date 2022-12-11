use std::iter::FusedIterator;
use std::num::NonZeroU32;

use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use tracing::warn;

use crate::slab::Slab;

#[derive(Clone, Debug)]
pub struct VersionedSlab<T> {
    slab: Slab<Slot<T>>,
    version: NonZeroU32,
}

#[derive(Clone, Debug)]
struct Slot<T> {
    value: T,
    version: NonZeroU32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Key {
    pub index: u32,
    pub version: NonZeroU32,
}

impl Key {
    pub const NULL: Self = Self {
        index: u32::MAX,
        version: match NonZeroU32::new(u32::MAX) {
            Some(n) => n,
            None => unreachable!(),
        },
    };

    pub fn new(index: u32, version: NonZeroU32) -> Self {
        Self { index, version }
    }
}

impl Default for Key {
    fn default() -> Self {
        Self::NULL
    }
}

impl Key {
    pub fn index(self) -> u32 {
        self.index
    }

    pub fn version(self) -> NonZeroU32 {
        self.version
    }
}

const ONE: NonZeroU32 = match NonZeroU32::new(1) {
    Some(n) => n,
    None => unreachable!(),
};

impl<T> VersionedSlab<T> {
    pub const fn new() -> Self {
        Self {
            slab: Slab::new(),
            version: ONE,
        }
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        let slot = self.slab.get(key.index as usize)?;
        (slot.version == key.version).then_some(&slot.value)
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        let slot = self.slab.get_mut(key.index as usize)?;
        (slot.version == key.version).then_some(&mut slot.value)
    }

    pub fn len(&self) -> usize {
        self.slab.len()
    }

    pub fn insert(&mut self, value: T) -> (Key, &mut T) {
        self.insert_with(|_| value)
    }

    pub fn insert_with(&mut self, f: impl FnOnce(Key) -> T) -> (Key, &mut T) {
        let version = self.version;
        self.version = NonZeroU32::new(version.get().wrapping_add(1)).unwrap_or_else(|| {
            warn!("slab version overflow");
            ONE
        });

        let (index, slot) = self.slab.insert_with(|index| {
            assert!(
                index < u32::MAX as usize,
                "too many values in versioned slab"
            );
            Slot {
                value: f(Key::new(index as u32, version)),
                version,
            }
        });

        (Key::new(index as u32, version), &mut slot.value)
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        self.get(key)?;
        Some(self.slab.remove(key.index as usize).unwrap().value)
    }

    pub fn retain(&mut self, mut f: impl FnMut(Key, &mut T) -> bool) {
        self.slab
            .retain(|idx, slot| f(Key::new(idx as u32, slot.version), &mut slot.value))
    }

    #[allow(unused)]
    pub fn clear(&mut self) {
        self.slab.clear();
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = (Key, &T)> + FusedIterator + Clone + '_ {
        self.slab
            .iter()
            .map(|(idx, slot)| (Key::new(idx as u32, slot.version), &slot.value))
    }

    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (Key, &mut T)> + FusedIterator + '_ {
        self.slab
            .iter_mut()
            .map(|(idx, slot)| (Key::new(idx as u32, slot.version), &mut slot.value))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (Key, &T)> + Clone + '_
    where
        T: Send + Sync,
    {
        self.slab
            .par_iter()
            .map(|(idx, slot)| (Key::new(idx as u32, slot.version), &slot.value))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (Key, &mut T)> + '_
    where
        T: Send + Sync,
    {
        self.slab
            .par_iter_mut()
            .map(|(idx, slot)| (Key::new(idx as u32, slot.version), &mut slot.value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_remove() {
        let mut slab = VersionedSlab::new();

        let k0 = slab.insert(10).0;
        let k1 = slab.insert(20).0;
        let k2 = slab.insert(30).0;
        assert!(k0 != k1 && k1 != k2 && k0 != k2);

        assert_eq!(slab.remove(k1), Some(20));
        assert_eq!(slab.get(k1), None);
        assert_eq!(slab.get(k2), Some(&30));
        let k3 = slab.insert(40).0;
        assert_eq!(slab.get(k0), Some(&10));
        assert_eq!(slab.get_mut(k3), Some(&mut 40));
        assert_eq!(slab.remove(k0), Some(10));

        slab.clear();
        assert_eq!(slab.len(), 0);
    }

    #[test]
    fn retain() {
        let mut sm = VersionedSlab::new();

        let k0 = sm.insert(10).0;
        let k1 = sm.insert(20).0;
        let k2 = sm.insert(30).0;

        sm.retain(|k, _| k == k1);

        assert_eq!(sm.get(k1), Some(&20));
        assert_eq!(sm.len(), 1);

        assert_eq!(sm.get(k0), None);
        assert_eq!(sm.get(k2), None);
    }
}
