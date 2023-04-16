use uuid::Uuid;

use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct SpectatorTeleportC2s {
    pub target: Uuid,
}
