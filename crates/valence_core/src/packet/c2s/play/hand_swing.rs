use crate::hand::Hand;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct HandSwingC2s {
    pub hand: Hand,
}
