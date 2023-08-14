use super::*;

const MAX_PAYLOAD_SIZE: usize = 0x100000;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CUSTOM_PAYLOAD_S2C)]
pub struct CustomPayloadS2c<'a> {
    pub channel: Ident<Cow<'a, str>>,
    pub data: Bounded<RawBytes<'a>, MAX_PAYLOAD_SIZE>,
}
