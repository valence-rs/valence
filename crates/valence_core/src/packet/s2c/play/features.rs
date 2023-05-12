use std::borrow::Cow;

use crate::ident::Ident;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct FeaturesS2c<'a> {
    pub features: Vec<Ident<Cow<'a, str>>>,
}
