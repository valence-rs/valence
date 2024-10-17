use std::borrow::Cow;

use valence_text::Text;

use super::set_objective_s2c::NumberFormat;
use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetScoreS2c<'a> {
    //The entity whose score this is. For players, this is their username; for other entities, it
    // is their UUID.
    pub entity_name: &'a str,
    pub objective_name: &'a str,
    pub value: VarInt,
    pub display_name: Option<Cow<'a, Text>>,
    pub number_format: Option<NumberFormat<'a>>,
}
