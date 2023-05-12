use std::borrow::Cow;

use crate::packet::{Decode, Encode};
use crate::text::Text;

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginDisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}
