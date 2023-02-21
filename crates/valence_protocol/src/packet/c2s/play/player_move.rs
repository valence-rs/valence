use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PositionAndOnGroundC2s {
    pub position: [f64; 3],
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct FullC2s {
    pub position: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct LookAndOnGroundC2s {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct OnGroundOnlyC2s {
    pub on_ground: bool,
}
