use std::borrow::Cow;

use uuid::Uuid;

use crate::types::Property;
use crate::username::Username;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x02]
pub struct LoginSuccessS2c<'a> {
    pub uuid: Uuid,
    pub username: Username<&'a str>,
    pub properties: Cow<'a, [Property]>,
}
