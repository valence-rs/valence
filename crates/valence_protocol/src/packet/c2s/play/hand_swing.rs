use crate::types::Hand;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct HandSwingC2s {
    pub hand: Hand,
}
