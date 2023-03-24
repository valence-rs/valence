use crate::item::ItemStack;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct ClickSlotC2s {
    pub window_id: u8,
    pub state_id: VarInt,
    pub slot_idx: i16,
    /// The button used to click the slot. An enum can't easily be used for this
    /// because the meaning of this value depends on the mode.
    pub button: i8,
    pub mode: ClickMode,
    pub slots: Vec<Slot>,
    pub carried_item: Option<ItemStack>,
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
pub struct Slot {
    pub idx: i16,
    pub item: Option<ItemStack>,
}
