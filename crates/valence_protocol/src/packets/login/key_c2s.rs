use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
/// Sent by the client to the server to respond to the [`HelloS2c`](crate::packets::login::HelloS2c)
/// packet. All proceeding packets will be encrypted.
pub struct KeyC2s<'a> {
    pub shared_secret: &'a [u8],
    pub verify_token: &'a [u8],
}
