use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct QueryPongS2c {
    pub payload: u64,
}
