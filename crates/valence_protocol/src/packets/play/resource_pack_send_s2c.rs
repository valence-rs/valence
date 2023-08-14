use super::*;

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::RESOURCE_PACK_SEND_S2C)]
pub struct ResourcePackSendS2c<'a> {
    pub url: &'a str,
    pub hash: Bounded<&'a str, 40>,
    pub forced: bool,
    pub prompt_message: Option<Cow<'a, Text>>,
}
