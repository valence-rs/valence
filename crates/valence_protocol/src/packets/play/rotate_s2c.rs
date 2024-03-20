use crate::{PacketSide, ByteAngle, Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(name = "ROTATE", side=PacketSide::Clientbound)]
pub struct RotateS2c {
    pub entity_id: VarInt,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}
