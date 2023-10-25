#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]

pub mod biome;
pub mod codec;
pub mod dimension_type;
pub mod tags;

use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
pub use biome::BiomeRegistry;
pub use codec::RegistryCodec;
pub use dimension_type::DimensionTypeRegistry;
use indexmap::map::Entry;
use indexmap::IndexMap;
pub use tags::TagsRegistry;
use valence_ident::Ident;

pub struct RegistryPlugin;

/// The [`SystemSet`] where the [`RegistryCodec`] and [`TagsRegistry`] caches
/// are rebuilt. Systems that modify the registry codec or tags registry should
/// run _before_ this.
///
/// This set lives in [`PostUpdate`].
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct RegistrySet;

impl Plugin for RegistryPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.configure_set(PostUpdate, RegistrySet);

        codec::build(app);
        tags::build(app);
    }
}

#[derive(Clone, Debug)]
pub struct Registry<I, V> {
    items: IndexMap<Ident<String>, V>,
    _marker: PhantomData<I>,
}

impl<I: RegistryIdx, V> Registry<I, V> {
    pub fn new() -> Self {
        Self {
            items: IndexMap::new(),
            _marker: PhantomData,
        }
    }

    pub fn insert(&mut self, name: impl Into<Ident<String>>, item: V) -> Option<I> {
        if self.items.len() >= I::MAX {
            // Too many items in the registry.
            return None;
        }

        let len = self.items.len();

        match self.items.entry(name.into()) {
            Entry::Occupied(_) => None,
            Entry::Vacant(ve) => {
                ve.insert(item);
                Some(I::from_index(len))
            }
        }
    }

    pub fn swap_to_front(&mut self, name: Ident<&str>) {
        if let Some(idx) = self.items.get_index_of(name.as_str()) {
            self.items.swap_indices(0, idx);
        }
    }

    pub fn remove(&mut self, name: Ident<&str>) -> Option<V> {
        self.items.shift_remove(name.as_str())
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn get(&self, name: Ident<&str>) -> Option<&V> {
        self.items.get(name.as_str())
    }

    pub fn get_mut(&mut self, name: Ident<&str>) -> Option<&mut V> {
        self.items.get_mut(name.as_str())
    }

    pub fn index_of(&self, name: Ident<&str>) -> Option<I> {
        self.items.get_index_of(name.as_str()).map(I::from_index)
    }

    pub fn iter(
        &self,
    ) -> impl DoubleEndedIterator<Item = (I, Ident<&str>, &V)> + ExactSizeIterator + '_ {
        self.items
            .iter()
            .enumerate()
            .map(|(i, (k, v))| (I::from_index(i), k.as_str_ident(), v))
    }

    pub fn iter_mut(
        &mut self,
    ) -> impl DoubleEndedIterator<Item = (I, Ident<&str>, &mut V)> + ExactSizeIterator + '_ {
        self.items
            .iter_mut()
            .enumerate()
            .map(|(i, (k, v))| (I::from_index(i), k.as_str_ident(), v))
    }
}

impl<I: RegistryIdx, V> Index<I> for Registry<I, V> {
    type Output = V;

    fn index(&self, index: I) -> &Self::Output {
        self.items
            .get_index(index.to_index())
            .unwrap_or_else(|| panic!("out of bounds registry index of {}", index.to_index()))
            .1
    }
}

impl<I: RegistryIdx, V> IndexMut<I> for Registry<I, V> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.items
            .get_index_mut(index.to_index())
            .unwrap_or_else(|| panic!("out of bounds registry index of {}", index.to_index()))
            .1
    }
}

impl<'a, I: RegistryIdx, V> Index<Ident<&'a str>> for Registry<I, V> {
    type Output = V;

    fn index(&self, index: Ident<&'a str>) -> &Self::Output {
        if let Some(item) = self.items.get(index.as_str()) {
            item
        } else {
            panic!("missing registry item with name '{index}'")
        }
    }
}

impl<'a, I: RegistryIdx, V> IndexMut<Ident<&'a str>> for Registry<I, V> {
    fn index_mut(&mut self, index: Ident<&'a str>) -> &mut Self::Output {
        if let Some(item) = self.items.get_mut(index.as_str()) {
            item
        } else {
            panic!("missing registry item with name '{index}'")
        }
    }
}

impl<I, V> Default for Registry<I, V> {
    fn default() -> Self {
        Self {
            items: IndexMap::new(),
            _marker: PhantomData,
        }
    }
}

pub trait RegistryIdx: Copy + Clone + PartialEq + Eq + PartialOrd + Ord + Hash + Debug {
    const MAX: usize;

    fn to_index(self) -> usize;
    fn from_index(idx: usize) -> Self;
}
