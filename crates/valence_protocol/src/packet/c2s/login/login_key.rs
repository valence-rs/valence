use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginKeyC2s<'a> {
    pub shared_secret: &'a [u8],
    pub verify_token: &'a [u8],
}
