use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginDisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}
