use std::borrow::Cow;

use crate::ident::Ident;
use crate::raw::RawBytes;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginQueryRequestS2c<'a> {
    pub message_id: VarInt,
    pub channel: Ident<Cow<'a, str>>,
    pub data: RawBytes<'a>,
}
