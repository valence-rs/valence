#![allow(dead_code)]

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::iter::FusedIterator;
use std::sync::Arc;

use flume::{Receiver, Sender};

#[derive(Debug)]
pub struct RcSlab<T> {
    entries: Vec<T>,
    free_send: Sender<usize>,
    free_recv: Receiver<usize>,
}

impl<T> RcSlab<T> {
    pub fn new() -> Self {
        let (free_send, free_recv) = flume::unbounded();

        Self {
            entries: vec![],
            free_send,
            free_recv,
        }
    }

    pub fn insert(&mut self, value: T) -> (Key, &mut T) {
        match self.free_recv.try_recv() {
            Ok(idx) => {
                self.entries[idx] = value;
                let k = Key(Arc::new(KeyInner {
                    index: idx,
                    free_send: self.free_send.clone(),
                }));

                (k, &mut self.entries[idx])
            }
            Err(_) => {
                let idx = self.entries.len();
                self.entries.push(value);
                let k = Key(Arc::new(KeyInner {
                    index: idx,
                    free_send: self.free_send.clone(),
                }));
                (k, &mut self.entries[idx])
            }
        }
    }

    pub fn get(&self, key: &Key) -> &T {
        &self.entries[key.0.index]
    }

    pub fn get_mut(&mut self, key: &Key) -> &mut T {
        &mut self.entries[key.0.index]
    }

    pub fn iter(&self) -> impl FusedIterator<Item = &T> + Clone + '_ {
        self.entries.iter()
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = &mut T> + '_ {
        self.entries.iter_mut()
    }
}

#[derive(Clone, Debug)]
pub struct Key(Arc<KeyInner>);

#[derive(Debug)]
struct KeyInner {
    index: usize,
    free_send: Sender<usize>,
}

impl Drop for KeyInner {
    fn drop(&mut self) {
        let _ = self.free_send.send(self.index);
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        Arc::as_ptr(&self.0) == Arc::as_ptr(&other.0)
    }
}

impl Eq for Key {}

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

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn rc_slab_insert() {
        let mut slab = RcSlab::new();
        let (k, v) = slab.insert(123);
        assert_eq!(*v, 123);

        let k2 = slab.insert(456).0;
        assert_ne!(k, k2);
        assert_eq!(slab.entries.len(), 2);

        drop(k);
        drop(k2);
        slab.insert(789);
        assert_eq!(slab.entries.len(), 2);
    }
}
