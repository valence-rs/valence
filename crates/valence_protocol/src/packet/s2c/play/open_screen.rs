use std::borrow::Cow;

use crate::text::Text;
use crate::types::WindowType;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct OpenScreenS2c<'a> {
    pub window_id: VarInt,
    pub window_type: WindowType,
    pub window_title: Cow<'a, Text>,
}
