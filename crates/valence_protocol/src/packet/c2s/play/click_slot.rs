use crate::item::ItemStack;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x0a]
pub struct ClickSlotC2s {
    pub window_id: u8,
    pub state_id: VarInt,
    pub slot_idx: i16,
    pub button: i8,
    pub mode: Mode,
    pub slots: Vec<Slot>,
    pub carried_item: Option<ItemStack>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum ClickContainerMode {
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
    pub stack: Option<ItemStack>,
}
