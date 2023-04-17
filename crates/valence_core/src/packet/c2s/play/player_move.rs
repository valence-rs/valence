use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PositionAndOnGround {
    pub position: [f64; 3],
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Full {
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct LookAndOnGround {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct OnGroundOnly {
    pub on_ground: bool,
}
