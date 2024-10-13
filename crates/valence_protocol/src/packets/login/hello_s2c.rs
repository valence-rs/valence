use crate::{Bounded, Decode, Encode, Packet, PacketState};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
/// Sent by the server to the client to initiate the login process.
pub struct HelloS2c<'a> {
    pub server_id: Bounded<&'a str, 20>,
    pub public_key: &'a [u8],
    pub verify_token: &'a [u8],
    pub should_authenticate: bool,
}
