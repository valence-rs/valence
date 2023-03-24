use crate::types::Hand;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerInteractEntityC2s {
    pub entity_id: VarInt,
    pub interact: EntityInteraction,
    pub sneaking: bool,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub enum EntityInteraction {
    Interact(Hand),
    Attack,
    InteractAt { target: [f32; 3], hand: Hand },
}
