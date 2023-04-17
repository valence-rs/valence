use bevy_ecs::prelude::*;

/// General-purpose reusable byte buffer.
///
/// No guarantees are made about the buffer's contents between systems.
/// Therefore, the inner `Vec` should be cleared before use.
#[derive(Component, Default, Debug)]
pub struct ScratchBuf(pub Vec<u8>);
