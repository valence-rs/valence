use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_LIST_HEADER_S2C)]
pub struct PlayerListHeaderS2c<'a> {
    pub header: Cow<'a, Text>,
    pub footer: Cow<'a, Text>,
}
