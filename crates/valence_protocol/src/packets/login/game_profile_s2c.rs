use std::borrow::Cow;

use uuid::Uuid;

use crate::profile::Property;
use crate::{Bounded, Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Login)]

/// Sent by the server to the client to send the game profile of the player.
/// This packet is sent after the client has successfully logged in. This packet
/// is used to send data about the player such as their UUID, username, skin,
/// and cape.
pub struct GameProfileS2c<'a> {
    pub uuid: Uuid,
    pub username: Bounded<&'a str, 16>,
    pub properties: Cow<'a, [Property<&'a str>]>,
    // This field was temporarily added in 1.20.5, it will be removed in 1.21.2
    pub strict_error_handling: bool,
}
