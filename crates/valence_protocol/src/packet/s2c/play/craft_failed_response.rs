use std::borrow::Cow;

use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct CraftFailedResponseS2c<'a> {
    pub window_id: u8,
    pub recipe: Ident<Cow<'a, str>>,
}
