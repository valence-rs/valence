use uuid::Uuid;

use crate::username::Username;
use crate::{Decode, DecodePacket, Encode, EncodePacket};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x00]
pub struct LoginHelloC2s<'a> {
    pub username: Username<&'a str>,
    pub profile_id: Option<Uuid>,
}
