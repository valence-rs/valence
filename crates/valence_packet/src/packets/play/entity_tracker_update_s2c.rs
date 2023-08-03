use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_TRACKER_UPDATE_S2C)]
pub struct EntityTrackerUpdateS2c<'a> {
    pub entity_id: VarInt,
    pub metadata: RawBytes<'a>,
}
