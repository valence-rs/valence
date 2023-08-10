use super::*;

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_REMOVE_S2C)]
pub struct PlayerRemoveS2c<'a> {
    pub uuids: Cow<'a, [Uuid]>,
}
