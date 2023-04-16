use crate::types::Hand;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct OpenWrittenBookS2c {
    pub hand: Hand,
}
