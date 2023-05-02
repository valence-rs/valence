use std::borrow::Cow;

use crate::ident::Ident;
use crate::packet::raw::RawBytes;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginQueryRequestS2c<'a> {
    pub message_id: VarInt,
    pub channel: Ident<Cow<'a, str>>,
    pub data: RawBytes<'a>,
}
