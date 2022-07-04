use std::iter::FusedIterator;
use std::mem;
use std::num::NonZeroU32;

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
    /// Version counter.
    version: NonZeroU32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Key {
    index: u32,
    version: NonZeroU32,
}

impl Key {
    pub const NULL: Self = Self {
        index: u32::MAX,
        version: match NonZeroU32::new(u32::MAX) {
            Some(n) => n,
            None => unreachable!(),
        },
    };
}

impl Default for Key {
    fn default() -> Self {
        Self::NULL
    }
}

impl Key {
    pub fn new(index: u32, version: NonZeroU32) -> Self {
        Self { index, version }
    }

    fn new_unique(index: u32, version: &mut NonZeroU32) -> Self {
        *version = NonZeroU32::new(version.get().wrapping_add(1)).unwrap_or_else(|| {
            log::warn!("slotmap version overflow");
            ONE
        });

        Self {
            index,
            version: *version,
        }
    }

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

#[derive(Clone, Debug)]
enum Slot<T> {
    Occupied { value: T, version: NonZeroU32 },
    Free { next_free: u32 },
}

impl<T> SlotMap<T> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            next_free_head: 0,
            count: 0,
            version: ONE,
        }
    }

    pub fn len(&self) -> usize {
        self.count as usize
    }

    pub fn insert(&mut self, val: T) -> (Key, &mut T) {
        self.insert_with(|_| val)
    }

    pub fn insert_with(&mut self, f: impl FnOnce(Key) -> T) -> (Key, &mut T) {
        assert!(self.count < u32::MAX, "SlotMap: too many items inserted");

        if self.next_free_head == self.slots.len() as u32 {
            self.count += 1;
            self.next_free_head += 1;

            let key = Key::new_unique(self.next_free_head - 1, &mut self.version);

            self.slots.push(Slot::Occupied {
                value: f(key),
                version: key.version(),
            });

            let value = match self.slots.last_mut() {
                Some(Slot::Occupied { value, .. }) => value,
                _ => unreachable!(),
            };
            (key, value)
        } else {
            let slot = &mut self.slots[self.next_free_head as usize];

            let next_free = match slot {
                Slot::Occupied { .. } => unreachable!("corrupt free list"),
                Slot::Free { next_free } => *next_free,
            };

            let key = Key::new_unique(self.next_free_head, &mut self.version);

            *slot = Slot::Occupied {
                value: f(key),
                version: key.version(),
            };

            let value = match slot {
                Slot::Occupied { value, .. } => value,
                Slot::Free { .. } => unreachable!(),
            };

            self.next_free_head = next_free;
            self.count += 1;

            (key, value)
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

    #[allow(unused)]
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

        let k0 = sm.insert(10).0;
        let k1 = sm.insert(20).0;
        let k2 = sm.insert(30).0;

        assert_eq!(sm.remove(k1), Some(20));
        assert_eq!(sm.get(k1), None);
        assert_eq!(sm.get(k2), Some(&30));
        let k3 = sm.insert(40).0;
        assert_eq!(sm.get(k0), Some(&10));
        assert_eq!(sm.get_mut(k3), Some(&mut 40));
        assert_eq!(sm.remove(k0), Some(10));

        sm.clear();
        assert_eq!(sm.len(), 0);
    }

    #[test]
    fn retain() {
        let mut sm = SlotMap::new();

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
