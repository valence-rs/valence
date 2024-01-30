use uuid::Uuid;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ResourcePackRemoveS2c {
    pub uuid: Option<Uuid>,
}
