use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAYER_SESSION_C2S)]
pub struct PlayerSessionC2s<'a> {
    pub session_id: Uuid,
    // Public key
    pub expires_at: i64,
    pub public_key_data: &'a [u8],
    pub key_signature: &'a [u8],
}
