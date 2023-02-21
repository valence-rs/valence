use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct KeepAliveS2c {
    pub id: u64,
}
