use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ScoreboardDisplayS2c<'a> {
    pub position: u8,
    pub score_name: &'a str,
}
