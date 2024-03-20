use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateTickRateS2c {
    pub tick_rate: f32,
    pub is_frozen: bool,
}