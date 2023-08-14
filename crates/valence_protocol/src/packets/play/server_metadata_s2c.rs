use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SERVER_METADATA_S2C)]
pub struct ServerMetadataS2c<'a> {
    pub motd: Cow<'a, Text>,
    pub icon: Option<&'a [u8]>,
    pub enforce_secure_chat: bool,
}
