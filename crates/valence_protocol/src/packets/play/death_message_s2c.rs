use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::DEATH_MESSAGE_S2C)]
pub struct DeathMessageS2c<'a> {
    pub player_id: VarInt,
    pub message: Cow<'a, Text>,
}
