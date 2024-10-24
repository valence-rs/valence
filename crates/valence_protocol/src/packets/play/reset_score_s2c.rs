use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ResetScoreS2c<'a> {
    //The entity whose score this is. For players, this is their username; for other entities, it
    // is their UUID.
    pub entity_name: &'a str,
    pub objective_name: Option<&'a str>,
}
