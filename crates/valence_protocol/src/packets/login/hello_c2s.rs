use uuid::Uuid;

use crate::{Bounded, Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]
/// Sent by the client to the server to initiate the login process.
pub struct HelloC2s<'a> {
    pub username: Bounded<&'a str, 16>,
    pub profile_id: Option<Uuid>,
}
