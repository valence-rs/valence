use bevy_ecs::prelude::Component;

/// The type use to represent a layer.
/// u32 is used to allow for +4 billion layers.
pub type LayerType = u32; // could be define behind a feature flag

/// Used in conjunction with the `ClientLayerSet` component.  
/// Example: you have `Layer(0)` on a cow, it will only be visible to clients
/// with the access to layer 0 in their `ClientLayerSet`.
#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct Layer(pub LayerType);
