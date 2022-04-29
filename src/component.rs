use std::any::{Any, TypeId};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};
use thiserror::Error;

/// Contains custom components
pub(crate) struct ComponentStore<I: Id> {
    ids: Vec<Slot>,
    next_free_head: u32,
    count: u32,
    components: HashMap<TypeId, Box<dyn ComponentVec>>,
    _marker: PhantomData<fn(I) -> I>,
}

impl<I: Id> ComponentStore<I> {
    pub fn new() -> Self {
        Self {
            ids: Vec::new(),
            next_free_head: 0,
            count: 0,
            components: HashMap::new(),
            _marker: PhantomData,
        }
    }

    pub fn count(&self) -> usize {
        self.count as usize
    }

    pub fn create_item(&mut self) -> I {
        assert!(self.count < u32::MAX - 1, "too many items");

        if self.next_free_head == self.ids.len() as u32 {
            self.ids.push(Slot {
                gen: ONE,
                next_free: None,
            });
            self.count += 1;
            self.next_free_head += 1;
            for v in self.components.values_mut() {
                v.push_default();
            }

            I::from_data(IdData {
                idx: self.next_free_head - 1,
                gen: ONE,
            })
        } else {
            let s = &mut self.ids[self.next_free_head as usize];
            s.gen = match NonZeroU32::new(s.gen.get().wrapping_add(1)) {
                Some(n) => n,
                None => {
                    log::warn!("generation overflow at idx = {}", self.next_free_head);
                    ONE
                }
            };
            let next_free = s.next_free.expect("corrupt free list");
            let id = I::from_data(IdData {
                idx: self.next_free_head,
                gen: s.gen,
            });

            self.next_free_head = next_free;
            self.count += 1;
            s.next_free = None;
            id
        }
    }

    pub fn delete_item(&mut self, id: I) -> bool {
        let id = id.to_data();
        match self.ids.get_mut(id.idx as usize) {
            Some(Slot {
                gen,
                next_free: nf @ None,
            }) if *gen == id.gen => {
                *nf = Some(self.next_free_head);
                self.next_free_head = id.idx;
                self.count -= 1;

                for vec in self.components.values_mut() {
                    vec.clear_at(id.idx as usize);
                }

                true
            }
            _ => false,
        }
    }

    pub fn is_valid(&self, id: I) -> bool {
        let id = id.to_data();
        match self.ids.get(id.idx as usize) {
            Some(Slot {
                gen,
                next_free: None,
            }) => *gen == id.gen,
            _ => false,
        }
    }

    pub fn get<Z: ZippedComponents<Id = I>>(&self, z: Z, id: I) -> Option<Z::Item> {
        if self.is_valid(id) {
            Some(z.raw_get(id.to_data().idx as usize))
        } else {
            None
        }
    }

    pub fn iter<'a, Z: ZippedComponents<Id = I> + 'a>(
        &'a self,
        z: Z,
    ) -> impl FusedIterator<Item = (I, Z::Item)> + 'a {
        self.ids
            .iter()
            .zip(z.raw_iter())
            .enumerate()
            .filter_map(|(i, (s, c))| {
                if s.next_free.is_none() {
                    Some((
                        I::from_data(IdData {
                            idx: i as u32,
                            gen: s.gen,
                        }),
                        c,
                    ))
                } else {
                    None
                }
            })
    }

    pub fn par_iter<'a, Z: ZippedComponents<Id = I> + 'a>(
        &'a self,
        z: Z,
    ) -> impl ParallelIterator<Item = (I, Z::Item)> + 'a {
        self.ids
            .par_iter()
            .zip(z.raw_par_iter())
            .enumerate()
            .filter_map(|(i, (s, c))| {
                if s.next_free.is_none() {
                    Some((
                        I::from_data(IdData {
                            idx: i as u32,
                            gen: s.gen,
                        }),
                        c,
                    ))
                } else {
                    None
                }
            })
    }

    pub fn ids(&self) -> impl FusedIterator<Item = I> + Clone + '_ {
        self.ids.iter().enumerate().filter_map(|(i, s)| {
            if s.next_free.is_none() {
                Some(I::from_data(IdData {
                    idx: i as u32,
                    gen: s.gen,
                }))
            } else {
                None
            }
        })
    }

    pub fn par_ids(&self) -> impl ParallelIterator<Item = I> + Clone + '_ {
        self.ids.par_iter().enumerate().filter_map(|(i, s)| {
            if s.next_free.is_none() {
                Some(I::from_data(IdData {
                    idx: i as u32,
                    gen: s.gen,
                }))
            } else {
                None
            }
        })
    }

    pub fn register_component<C: 'static + Send + Sync + DefaultPrivate>(&mut self) {
        if let Entry::Vacant(ve) = self.components.entry(TypeId::of::<C>()) {
            let mut vec = Vec::new();
            vec.resize_with(self.ids.len(), C::default_private);
            ve.insert(Box::new(RwLock::new(vec)));
        }
    }

    pub fn unregister_component<C: 'static + Send + Sync + DefaultPrivate>(&mut self) {
        self.components.remove(&TypeId::of::<C>());
    }

    pub fn is_registered<C: 'static + Send + Sync + DefaultPrivate>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<C>())
    }

    pub fn components<C: 'static + Send + Sync + DefaultPrivate>(
        &self,
    ) -> Result<Components<C, I>, Error> {
        let handle = self
            .components
            .get(&TypeId::of::<C>())
            .ok_or(Error::UnknownComponent)?
            .as_any()
            .downcast_ref::<RwLock<Vec<C>>>()
            .unwrap()
            .try_read()
            .ok_or(Error::NoReadAccess)?;

        Ok(Components {
            handle,
            _marker: PhantomData,
        })
    }

    pub fn components_mut<C: 'static + Send + Sync + DefaultPrivate>(
        &self,
    ) -> Result<ComponentsMut<C, I>, Error> {
        let handle = self
            .components
            .get(&TypeId::of::<C>())
            .ok_or(Error::UnknownComponent)?
            .as_any()
            .downcast_ref::<RwLock<Vec<C>>>()
            .unwrap()
            .try_write()
            .ok_or(Error::NoWriteAccess)?;

        Ok(ComponentsMut {
            handle,
            _marker: PhantomData,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    gen: NonZeroU32,
    next_free: Option<u32>,
}

pub trait Id: IdRaw + Copy + Send + Sync {}

const ONE: NonZeroU32 = match NonZeroU32::new(1) {
    Some(n) => n,
    None => unreachable!(),
};

trait ComponentVec: Any + Send + Sync {
    fn push_default(&mut self);

    fn clear_at(&mut self, idx: usize);

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static + Send + Sync + DefaultPrivate> ComponentVec for RwLock<Vec<T>> {
    fn push_default(&mut self) {
        self.get_mut().push(T::default_private());
    }

    fn clear_at(&mut self, idx: usize) {
        self.get_mut()[idx] = T::default_private();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct Components<'a, C: 'static + Send + Sync, I: Id> {
    handle: RwLockReadGuard<'a, Vec<C>>,
    _marker: PhantomData<fn(I) -> I>,
}

impl<'a, 'b, C: 'static + Send + Sync, I: Id> ZippedComponentsRaw for &'b Components<'a, C, I> {
    type RawItem = &'b C;
    type RawIter = std::slice::Iter<'b, C>;
    type RawParIter = rayon::slice::Iter<'b, C>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.handle[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.handle.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.handle.par_iter()
    }
}

impl<'a, 'b, C: 'static + Send + Sync, I: Id> ZippedComponents for &'b Components<'a, C, I> {
    type Id = I;
    type Item = &'b C;
}

pub struct ComponentsMut<'a, C: 'static + Send + Sync, I: Id> {
    handle: RwLockWriteGuard<'a, Vec<C>>,
    _marker: PhantomData<fn(I) -> I>,
}

impl<'a, 'b, C: 'static + Send + Sync, I: Id> ZippedComponentsRaw for &'b ComponentsMut<'a, C, I> {
    type RawItem = &'b C;
    type RawIter = std::slice::Iter<'b, C>;
    type RawParIter = rayon::slice::Iter<'b, C>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.handle[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.handle.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.handle.par_iter()
    }
}

impl<'a, 'b, C: 'static + Send + Sync, I: Id> ZippedComponents for &'b ComponentsMut<'a, C, I> {
    type Id = I;
    type Item = &'b C;
}

impl<'a, 'b, C: 'static + Send + Sync, I: Id> ZippedComponentsRaw
    for &'b mut ComponentsMut<'a, C, I>
{
    type RawItem = &'b mut C;
    type RawIter = std::slice::IterMut<'b, C>;
    type RawParIter = rayon::slice::IterMut<'b, C>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &mut self.handle[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.handle.iter_mut()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.handle.par_iter_mut()
    }
}

impl<'a, 'b, C: 'static + Send + Sync, I: Id> ZippedComponents for &'b mut ComponentsMut<'a, C, I> {
    type Id = I;
    type Item = &'b mut C;
}

/// The possible errors when requesting a component.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Error)]
pub enum Error {
    #[error("an unknown component type was requested")]
    UnknownComponent,
    #[error("shared access to a component was requested while exclusive access was already held")]
    NoReadAccess,
    #[error(
        "exclusive access to a component was requested while shared or exclusive access was \
         already held"
    )]
    NoWriteAccess,
}

pub(crate) mod private {
    use super::*;

    pub trait ZippedComponentsRaw {
        type RawItem: Send + Sync;
        type RawIter: FusedIterator<Item = Self::RawItem>;
        type RawParIter: IndexedParallelIterator<Item = Self::RawItem>;

        fn raw_get(self, idx: usize) -> Self::RawItem;
        fn raw_iter(self) -> Self::RawIter;
        fn raw_par_iter(self) -> Self::RawParIter;
    }

    pub trait IdRaw {
        fn to_data(self) -> IdData;
        fn from_data(id: IdData) -> Self;
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub struct IdData {
        pub idx: u32,
        pub gen: NonZeroU32,
    }

    impl IdData {
        pub const NULL: IdData = IdData {
            idx: u32::MAX,
            gen: match NonZeroU32::new(u32::MAX) {
                Some(n) => n,
                None => unreachable!(),
            },
        };
    }

    impl Default for IdData {
        fn default() -> Self {
            Self::NULL
        }
    }

    #[derive(Clone, Debug)]
    pub struct MultiZip<T> {
        pub tuple: T,
    }

    impl<T> MultiZip<T> {
        pub fn new(tuple: T) -> Self {
            Self { tuple }
        }
    }
}

pub(crate) use private::*;

/// Like `Default`, but only usable internally by this crate.
///
/// This prevents invariants regarding built-in components from being broken
/// by library users.
pub(crate) trait DefaultPrivate {
    fn default_private() -> Self;
}

impl<T: Default> DefaultPrivate for T {
    fn default_private() -> Self {
        T::default()
    }
}

pub trait ZippedComponents: ZippedComponentsRaw<RawItem = Self::Item> {
    type Id: Copy;
    type Item: Send + Sync;
}

macro_rules! tuple_impl {
    ($($T:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($T: ZippedComponentsRaw,)*> ZippedComponentsRaw for ($($T,)*) {
            type RawItem = ($($T::RawItem,)*);
            type RawIter = MultiZip<($($T::RawIter,)*)>;
            type RawParIter = rayon::iter::MultiZip<($($T::RawParIter,)*)>;

            fn raw_get(self, idx: usize) -> Self::RawItem {
                let ($($T,)*) = self;
                ($($T.raw_get(idx),)*)
            }

            fn raw_iter(self) -> Self::RawIter {
                let ($($T,)*) = self;
                MultiZip::new(($($T.raw_iter(),)*))
            }

            fn raw_par_iter(self) -> Self::RawParIter {
                let ($($T,)*) = self;
                ($($T.raw_par_iter(),)*).into_par_iter()
            }
        }


        #[allow(non_snake_case)]
        impl<$($T: Iterator,)*> Iterator for MultiZip<($($T,)*)> {
            type Item = ($($T::Item,)*);

            fn next(&mut self) -> Option<Self::Item> {
                let ($($T,)*) = &mut self.tuple;
                Some(($($T.next()?,)*))
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                let lower = usize::MAX;
                let upper: Option<usize> = None;
                let ($($T,)*) = &self.tuple;
                $(
                    let (l, u) = $T.size_hint();
                    let lower = lower.min(l);
                    let upper = match (upper, u) {
                        (Some(l), Some(r)) => Some(l.min(r)),
                        (Some(l), None) => Some(l),
                        (None, Some(r)) => Some(r),
                        (None, None) => None
                    };
                )*
                (lower, upper)
            }
        }

        #[allow(non_snake_case)]
        impl<$($T: ExactSizeIterator,)*> ExactSizeIterator for MultiZip<($($T,)*)> {
            fn len(&self) -> usize {
                let len = usize::MAX;
                let ($($T,)*) = &self.tuple;
                $(
                    let len = len.min($T.len());
                )*

                debug_assert_eq!(self.size_hint(), (len, Some(len)));
                len
            }
        }

        #[allow(non_snake_case)]
        impl<$($T: DoubleEndedIterator + ExactSizeIterator,)*> DoubleEndedIterator for MultiZip<($($T,)*)> {
            fn next_back(&mut self) -> Option<Self::Item> {
                let len = self.len();
                let ($($T,)*) = &mut self.tuple;

                $(
                    let this_len = $T.len();
                    debug_assert!(this_len >= len);
                    for _ in 0..this_len - len {
                        $T.next_back();
                    }
                    let $T = $T.next_back();
                )*

                Some(($($T?,)*))
            }
        }

        impl<$($T: FusedIterator,)*> FusedIterator for MultiZip<($($T,)*)> {}
    }
}

tuple_impl!(A);
tuple_impl!(A, B);
tuple_impl!(A, B, C);
tuple_impl!(A, B, C, D);
tuple_impl!(A, B, C, D, E);
tuple_impl!(A, B, C, D, E, F);
tuple_impl!(A, B, C, D, E, F, G);
tuple_impl!(A, B, C, D, E, F, G, H);
tuple_impl!(A, B, C, D, E, F, G, H, I);
tuple_impl!(A, B, C, D, E, F, G, H, I, J);
tuple_impl!(A, B, C, D, E, F, G, H, I, J, K);
tuple_impl!(A, B, C, D, E, F, G, H, I, J, K, L);
