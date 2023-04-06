use std::borrow::Cow;

use crate::ident::Ident;
use crate::raw::RawBytes;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct CustomPayloadS2c<'a> {
    pub channel: Ident<Cow<'a, str>>,
    pub data: RawBytes<'a>,
}
