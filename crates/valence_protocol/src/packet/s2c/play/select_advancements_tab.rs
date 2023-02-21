use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct SelectAdvancementsTabS2c<'a> {
    pub identifier: Option<Ident<&'a str>>,
}
