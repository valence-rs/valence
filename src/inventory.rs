use std::iter::FusedIterator;
use std::mem;
use std::num::Wrapping;
use std::ops::{Deref, DerefMut};

use valence_protocol::packets::s2c::play::SetContainerSlotEncode;
use valence_protocol::{InventoryKind, ItemStack, Text, VarInt};

use crate::config::Config;
use crate::server::PlayPacketSender;
use crate::slab_versioned::{Key, VersionedSlab};

pub struct Inventories<C: Config> {
    slab: VersionedSlab<Inventory<C>>,
}

impl<C: Config> Inventories<C> {
    pub(crate) fn new() -> Self {
        Self {
            slab: VersionedSlab::new(),
        }
    }

    pub fn insert(
        &mut self,
        kind: InventoryKind,
        title: impl Into<Text>,
        state: C::InventoryState,
    ) -> (InventoryId, &mut Inventory<C>) {
        let (id, inv) = self.slab.insert(Inventory {
            state,
            title: title.into(),
            kind,
            slots: vec![None; kind.slot_count()].into(),
            modified: 0,
        });

        (InventoryId(id), inv)
    }

    pub fn remove(&mut self, id: InventoryId) -> Option<C::InventoryState> {
        self.slab.remove(id.0).map(|inv| inv.state)
    }

    pub fn get(&self, id: InventoryId) -> Option<&Inventory<C>> {
        self.slab.get(id.0)
    }

    pub fn get_mut(&mut self, id: InventoryId) -> Option<&mut Inventory<C>> {
        self.slab.get_mut(id.0)
    }

    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (InventoryId, &Inventory<C>)> + FusedIterator + Clone + '_
    {
        self.slab.iter().map(|(k, inv)| (InventoryId(k), inv))
    }

    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (InventoryId, &mut Inventory<C>)> + FusedIterator + '_ {
        self.slab.iter_mut().map(|(k, inv)| (InventoryId(k), inv))
    }

    pub(crate) fn update(&mut self) {
        for (_, inv) in self.iter_mut() {
            inv.modified = 0;
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct InventoryId(Key);

impl InventoryId {
    pub const NULL: Self = Self(Key::NULL);
}

pub struct Inventory<C: Config> {
    /// Custom state
    pub state: C::InventoryState,
    title: Text,
    kind: InventoryKind,
    slots: Box<[Option<ItemStack>]>,
    /// Contains a set bit for each modified slot in `slots`.
    modified: u64,
}

impl<C: Config> Deref for Inventory<C> {
    type Target = C::InventoryState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<C: Config> DerefMut for Inventory<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<C: Config> Inventory<C> {
    pub fn slot(&self, idx: u16) -> Option<&ItemStack> {
        self.slots
            .get(idx as usize)
            .expect("slot index out of range")
            .as_ref()
    }

    pub fn replace_slot(
        &mut self,
        idx: u16,
        item: impl Into<Option<ItemStack>>,
    ) -> Option<ItemStack> {
        assert!(idx < self.slot_count(), "slot index out of range");

        let new = item.into();
        let old = &mut self.slots[idx as usize];

        if new != *old {
            self.modified |= 1 << idx;
        }

        mem::replace(old, new)
    }

    pub fn slot_count(&self) -> u16 {
        self.slots.len() as u16
    }

    pub fn slots(
        &self,
    ) -> impl ExactSizeIterator<Item = Option<&ItemStack>>
           + DoubleEndedIterator
           + FusedIterator
           + Clone
           + '_ {
        self.slots.iter().map(|item| item.as_ref())
    }

    pub fn kind(&self) -> InventoryKind {
        self.kind
    }

    pub fn title(&self) -> &Text {
        &self.title
    }

    pub fn replace_title(&mut self, title: impl Into<Text>) -> Text {
        // TODO: set title modified flag
        mem::replace(&mut self.title, title.into())
    }

    pub(crate) fn slot_slice(&self) -> &[Option<ItemStack>] {
        self.slots.as_ref()
    }

    pub(crate) fn send_update(
        &self,
        send: &mut PlayPacketSender,
        window_id: u8,
        state_id: &mut Wrapping<i32>,
    ) -> anyhow::Result<()> {
        if self.modified != 0 {
            for (idx, slot) in self.slots.iter().enumerate() {
                if (self.modified >> idx) & 1 == 1 {
                    *state_id += 1;

                    send.append_packet(&SetContainerSlotEncode {
                        window_id: window_id as i8,
                        state_id: VarInt(state_id.0),
                        slot_idx: idx as i16,
                        slot_data: slot.as_ref(),
                    })?;
                }
            }
        }

        Ok(())
    }
}
