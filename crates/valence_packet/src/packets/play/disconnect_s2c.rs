use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::DISCONNECT_S2C)]
pub struct DisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}
