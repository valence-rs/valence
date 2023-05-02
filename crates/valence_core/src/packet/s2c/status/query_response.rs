use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct QueryResponseS2c<'a> {
    pub json: &'a str,
}
