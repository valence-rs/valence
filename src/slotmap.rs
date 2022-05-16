//! Like the `slotmap` crate, but uses no unsafe code and has rayon support.

use std::iter::FusedIterator;
use std::mem;
use std::num::{NonZeroU32, NonZeroU64};
use std::sync::atomic::{AtomicU64, Ordering};

use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

#[derive(Clone, Debug)]
pub struct SlotMap<T> {
    slots: Vec<Slot<T>>,
    /// Top of the free stack.
    next_free_head: u32,
    /// The number of occupied slots.
    count: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Key {
    index: u32,
    // Split the u64 version into two u32 fields so that the key is 12 bytes on 64 bit systems.
    version_high: NonZeroU32,
    version_low: u32,
}

impl Key {
    fn new(index: u32, version: NonZeroU64) -> Self {
        Self {
            index,
            version_high: NonZeroU32::new((version.get() >> 32) as u32)
                .expect("versions <= 0x00000000ffffffff are illegal"),
            version_low: version.get() as u32,
        }
    }

    fn new_unique(index: u32) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(u64::MAX);

        let version = NEXT.fetch_sub(1, Ordering::SeqCst);
        Self {
            index,
            version_high: ((version >> 32) as u32).try_into().unwrap(),
            version_low: version as u32,
        }
    }

    pub fn index(self) -> u32 {
        self.index
    }

    pub fn version(self) -> NonZeroU64 {
        let n = (self.version_high.get() as u64) << 32 | self.version_low as u64;
        NonZeroU64::new(n).expect("version should be nonzero")
    }
}

#[derive(Clone, Debug)]
enum Slot<T> {
    Occupied { value: T, version: NonZeroU64 },
    Free { next_free: u32 },
}

impl<T> SlotMap<T> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            next_free_head: 0,
            count: 0,
        }
    }

    pub fn count(&self) -> usize {
        self.count as usize
    }

    pub fn insert(&mut self, val: T) -> Key {
        self.insert_with(|_| val)
    }

    pub fn insert_with(&mut self, f: impl FnOnce(Key) -> T) -> Key {
        assert!(self.count < u32::MAX, "SlotMap: too many items inserted");

        if self.next_free_head == self.slots.len() as u32 {
            self.count += 1;
            self.next_free_head += 1;

            let key = Key::new_unique(self.next_free_head - 1);

            self.slots.push(Slot::Occupied {
                value: f(key),
                version: key.version(),
            });
            key
        } else {
            let slot = &mut self.slots[self.next_free_head as usize];

            let next_free = match slot {
                Slot::Occupied { .. } => unreachable!("corrupt free list"),
                Slot::Free { next_free } => *next_free,
            };

            let key = Key::new_unique(self.next_free_head);

            *slot = Slot::Occupied {
                value: f(key),
                version: key.version(),
            };

            self.next_free_head = next_free;
            self.count += 1;
            key
        }
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        let slot = self.slots.get_mut(key.index as usize)?;
        match slot {
            Slot::Occupied { version, .. } if *version == key.version() => {
                let old_slot = mem::replace(
                    slot,
                    Slot::Free {
                        next_free: self.next_free_head,
                    },
                );

                self.next_free_head = key.index;
                self.count -= 1;

                match old_slot {
                    Slot::Occupied { value, .. } => Some(value),
                    Slot::Free { .. } => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        match self.slots.get(key.index as usize)? {
            Slot::Occupied { value, version } if *version == key.version() => Some(value),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        match self.slots.get_mut(key.index as usize)? {
            Slot::Occupied { value, version } if *version == key.version() => Some(value),
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.next_free_head = 0;
        self.count = 0;
    }

    pub fn retain(&mut self, mut f: impl FnMut(Key, &mut T) -> bool) {
        for (i, mut slot) in self.slots.iter_mut().enumerate() {
            if let Slot::Occupied { value, version } = &mut slot {
                let key = Key::new(i as u32, *version);

                if !f(key, value) {
                    *slot = Slot::Free {
                        next_free: self.next_free_head,
                    };

                    self.next_free_head = key.index;
                    self.count -= 1;
                }
            }
        }
    }

    pub fn iter(&self) -> impl FusedIterator<Item = (Key, &T)> + Clone + '_ {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| match &slot {
                Slot::Occupied { value, version } => Some((Key::new(i as u32, *version), value)),
                Slot::Free { .. } => None,
            })
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (Key, &mut T)> + '_ {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(i, slot)| match slot {
                Slot::Occupied { value, version } => Some((Key::new(i as u32, *version), value)),
                Slot::Free { .. } => None,
            })
    }
}

impl<T: Sync> SlotMap<T> {
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (Key, &T)> + Clone + '_ {
        self.slots
            .par_iter()
            .enumerate()
            .filter_map(|(i, slot)| match &slot {
                Slot::Occupied { value, version } => Some((Key::new(i as u32, *version), value)),
                Slot::Free { .. } => None,
            })
    }
}

impl<T: Send + Sync> SlotMap<T> {
    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (Key, &mut T)> + '_ {
        self.slots
            .par_iter_mut()
            .enumerate()
            .filter_map(|(i, slot)| match slot {
                Slot::Occupied { value, version } => Some((Key::new(i as u32, *version), value)),
                Slot::Free { .. } => None,
            })
    }
}

impl<T> Default for SlotMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_remove() {
        let mut sm = SlotMap::new();

        let k0 = sm.insert(10);
        let k1 = sm.insert(20);
        let k2 = sm.insert(30);

        assert_eq!(sm.remove(k1), Some(20));
        assert_eq!(sm.get(k1), None);
        assert_eq!(sm.get(k2), Some(&30));
        let k3 = sm.insert(40);
        assert_eq!(sm.get(k0), Some(&10));
        assert_eq!(sm.get_mut(k3), Some(&mut 40));
        assert_eq!(sm.remove(k0), Some(10));

        sm.clear();
        assert_eq!(sm.count(), 0);
    }

    #[test]
    fn retain() {
        let mut sm = SlotMap::new();

        let k0 = sm.insert(10);
        let k1 = sm.insert(20);
        let k2 = sm.insert(30);

        sm.retain(|k, _| k == k1);

        assert_eq!(sm.get(k1), Some(&20));
        assert_eq!(sm.count(), 1);

        assert_eq!(sm.get(k0), None);
        assert_eq!(sm.get(k2), None);
    }

    #[test]
    #[should_panic]
    fn bad_key() {
        let _ = Key::new(0, NonZeroU64::new(0x00000000ffffffff).unwrap());
    }
}
