use bevy_ecs::prelude::*;
use crate::dimension::DimensionId;

#[derive(Default, Component)]
pub struct Instance {
    dimension: DimensionId,
    /// Packet data to send to all clients in this instance at the end of the tick.
    packet_buffer: Vec<u8>,
}

impl Instance {
    pub fn new(dimension: DimensionId) -> Self {
        Self {
            dimension,
            packet_buffer: vec![]
        }
    }

    pub fn dimension(&self) -> DimensionId {
        self.dimension
    }
}
