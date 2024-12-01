use std::borrow::Cow;

use crate::{Decode, Encode, ItemStack, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ContainerClickC2s<'a> {
    pub window_id: VarInt,
    pub state_id: VarInt,
    pub slot_idx: i16,
    /// The button used to click the slot. An enum can't easily be used for this
    /// because the meaning of this value depends on the mode.
    pub button: i8,
    pub mode: ClickMode,
    pub slot_changes: Cow<'a, [SlotChange<'a>]>,
    pub carried_item: ItemStack<'a>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum ClickMode {
    Click,
    ShiftClick,
    Hotbar,
    CreativeMiddleClick,
    DropKey,
    Drag,
    DoubleClick,
}

#[derive(Clone, Debug, Encode, Decode)]
pub struct SlotChange<'a> {
    pub idx: i16,
    pub stack: ItemStack<'a>,
}
