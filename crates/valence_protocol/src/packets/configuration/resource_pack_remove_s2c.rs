use uuid::Uuid;

use crate::{Decode, Encode, Packet, PacketState};

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(state = PacketState::Configuration)]
pub struct ResourcePackRemoveS2c {
    pub uuid: Option<Uuid>,
}
