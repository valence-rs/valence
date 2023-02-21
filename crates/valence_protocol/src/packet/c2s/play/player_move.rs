use crate::{Decode, DecodePacket, Encode, EncodePacket};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x13]
pub struct PositionAndOnGroundC2s {
    pub position: [f64; 3],
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x14]
pub struct FullC2s {
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x15]
pub struct LookAndOnGroundC2s {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x16]
pub struct OnGroundOnlyC2s {
    pub on_ground: bool,
}
