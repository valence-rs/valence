use super::*;

pub const MAX_PAYLOAD_SIZE: usize = 32767;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CUSTOM_PAYLOAD_C2S)]
pub struct CustomPayloadC2s<'a> {
    pub channel: Ident<Cow<'a, str>>,
    pub data: Bounded<RawBytes<'a>, MAX_PAYLOAD_SIZE>,
}
