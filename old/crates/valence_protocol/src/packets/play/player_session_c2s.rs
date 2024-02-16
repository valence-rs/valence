use uuid::Uuid;

use crate::{Bounded, Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct PlayerSessionC2s<'a> {
    pub session_id: Uuid,
    // Public key
    pub expires_at: i64,
    pub public_key_data: Bounded<&'a [u8], 512>,
    pub key_signature: Bounded<&'a [u8], 4096>,
}
