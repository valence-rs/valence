use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_INTERACT_ENTITY_C2S)]
pub struct PlayerInteractEntityC2s {
    pub entity_id: VarInt,
    pub interact: EntityInteraction,
    pub sneaking: bool,
}

#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub enum EntityInteraction {
    Interact(Hand),
    Attack,
    InteractAt { target: Vec3, hand: Hand },
}
