use crate::types::Hand;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x0f]
pub struct PlayerInteractC2s {
    pub entity_id: VarInt,
    pub interact: Interaction,
    pub sneaking: bool,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub enum Interaction {
    Interact(Hand),
    Attack,
    InteractAt { target: [f32; 3], hand: Hand },
}
