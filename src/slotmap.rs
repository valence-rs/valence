//! Like the `slotmap` crate, but uses no unsafe code and has rayon support.

use std::iter::FusedIterator;
use std::mem;
use std::num::NonZeroU32;

use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};

#[derive(Clone, Debug)]
pub struct SlotMap<T> {
    slots: Vec<Slot<T>>,
    next_free_head: u32,
    /// The number of occupied slots.
    count: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Key {
    version: NonZeroU32,
    index: u32,
}

impl Key {
    pub fn version(self) -> NonZeroU32 {
        self.version
    }

    pub fn index(self) -> u32 {
        self.index
    }
}

#[derive(Clone, Debug)]
struct Slot<T> {
    version: NonZeroU32,
    item: Item<T>,
}

#[derive(Clone, Debug)]
enum Item<T> {
    Occupied(T),
    Vacant { next_free: u32 },
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
        assert!(
            // -1 so that NULL is always invalid.
            self.count < u32::MAX,
            "SlotMap: too many items inserted"
        );

        if self.next_free_head == self.slots.len() as u32 {
            self.slots.push(Slot {
                version: ONE,
                item: Item::Occupied(val),
            });

            self.count += 1;
            self.next_free_head += 1;
            Key {
                version: ONE,
                index: self.next_free_head - 1,
            }
        } else {
            let slot = &mut self.slots[self.next_free_head as usize];
            slot.version = match NonZeroU32::new(slot.version.get().wrapping_add(1)) {
                Some(n) => n,
                None => {
                    log::debug!("SlotMap: version overflow at idx = {}", self.next_free_head);
                    ONE
                }
            };

            let next_free = match slot.item {
                Item::Occupied(_) => unreachable!("corrupt free list"),
                Item::Vacant { next_free } => next_free,
            };

            let key = Key {
                version: slot.version,
                index: self.next_free_head,
            };

            self.next_free_head = next_free;
            self.count += 1;
            slot.item = Item::Occupied(val);
            key
        }
    }

    pub fn remove(&mut self, key: Key) -> Option<T> {
        let Slot { version, item } = self.slots.get_mut(key.index as usize)?;

        match item {
            Item::Occupied(_) if *version == key.version => {
                let item = mem::replace(
                    item,
                    Item::Vacant {
                        next_free: self.next_free_head,
                    },
                );

                self.next_free_head = key.index;
                self.count -= 1;

                match item {
                    Item::Occupied(val) => Some(val),
                    Item::Vacant { next_free } => unreachable!(),
                }
            }
            _ => None,
        }
    }

    pub fn get(&self, key: Key) -> Option<&T> {
        match self.slots.get(key.index as usize)? {
            Slot {
                version,
                item: Item::Occupied(val),
            } if *version == key.version => Some(val),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, key: Key) -> Option<&mut T> {
        match self.slots.get_mut(key.index as usize)? {
            Slot {
                version,
                item: Item::Occupied(val),
            } if *version == key.version => Some(val),
            _ => None,
        }
    }

    pub fn key_at_index(&self, idx: usize) -> Option<Key> {
        Some(Key {
            version: self.slots.get(idx)?.version,
            index: idx as u32,
        })
    }

    pub fn clear(&mut self) {
        self.slots.clear();
        self.next_free_head = 0;
        self.count = 0;
    }

    pub fn retain(&mut self, mut f: impl FnMut(Key, &mut T) -> bool) {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if let Item::Occupied(val) = &mut slot.item {
                let key = Key {
                    version: slot.version,
                    index: i as u32,
                };
                if !f(key, val) {
                    slot.item = Item::Vacant {
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
            .filter_map(|(i, slot)| match &slot.item {
                Item::Occupied(val) => Some((
                    Key {
                        version: slot.version,
                        index: i as u32,
                    },
                    val,
                )),
                Item::Vacant { .. } => None,
            })
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (Key, &mut T)> + '_ {
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(|(i, slot)| match &mut slot.item {
                Item::Occupied(val) => Some((
                    Key {
                        version: slot.version,
                        index: i as u32,
                    },
                    val,
                )),
                Item::Vacant { .. } => None,
            })
    }
}

impl<T: Sync> SlotMap<T> {
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (Key, &T)> + Clone + '_ {
        self.slots
            .par_iter()
            .enumerate()
            .filter_map(|(i, slot)| match &slot.item {
                Item::Occupied(val) => Some((
                    Key {
                        version: slot.version,
                        index: i as u32,
                    },
                    val,
                )),
                Item::Vacant { .. } => None,
            })
    }
}

impl<T: Send + Sync> SlotMap<T> {
    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (Key, &mut T)> + '_ {
        self.slots
            .par_iter_mut()
            .enumerate()
            .filter_map(|(i, slot)| match &mut slot.item {
                Item::Occupied(val) => Some((
                    Key {
                        version: slot.version,
                        index: i as u32,
                    },
                    val,
                )),
                Item::Vacant { .. } => None,
            })
    }
}

impl<T> Default for SlotMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

const ONE: NonZeroU32 = match NonZeroU32::new(1) {
    Some(n) => n,
    None => unreachable!(),
};

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
}
