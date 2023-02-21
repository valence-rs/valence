use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct FeaturesS2c<'a> {
    pub features: Vec<Ident<&'a str>>,
}
