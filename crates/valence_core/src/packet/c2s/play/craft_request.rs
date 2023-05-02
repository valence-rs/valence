use std::borrow::Cow;

use crate::ident::Ident;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct CraftRequestC2s<'a> {
    pub window_id: i8,
    pub recipe: Ident<Cow<'a, str>>,
    pub make_all: bool,
}
