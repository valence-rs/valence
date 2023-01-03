use bevy_ecs::prelude::*;
use crate::dimension::DimensionId;

#[derive(Default, Component)]
pub struct Instance {
    dimension: DimensionId,
    packet_buffer: Vec<u8>,
}

impl Instance {
    pub fn new(dimension: DimensionId) -> Self {
        Self {
            dimension,
            packet_buffer: vec![]
        }
    }
}
