use crate::types::MessageSignature;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct RemoveMessageS2c<'a> {
    pub signature: MessageSignature<'a>,
}
