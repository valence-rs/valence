use std::borrow::Cow;

use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct SelectAdvancementTabS2c<'a> {
    pub identifier: Option<Ident<Cow<'a, str>>>,
}
