use uuid::Uuid;

use crate::{Decode, Encode, Packet};
//Teleports the player to the given entity. The player must be in spectator
// mode.
#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct TeleportToEntityC2s {
    pub target: Uuid,
}
