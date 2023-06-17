use std::collections::HashSet;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_core::layer::LayerType;

use crate::FlushPacketsSet;

pub(super) fn build(app: &mut App) {
    app.add_system(update_old_client_layer_set.in_set(FlushPacketsSet));
}

/// A component that represents the layers that an client is on.
/// This is used to determine which entity to send to a client.
///
/// Usage: `ClientLayerSet::new(vec![1, 2])`
///
/// This will create a new `ClientLayerSet` with layers 1 and 2.
/// The entity that are on layer 1 and 2 will be sent to the client.
/// as well as the entity that don't have a `Layer` component.
#[derive(Component, Default, Debug)]
pub struct ClientLayerSet {
    pub layers: HashSet<LayerType>,
    old_layers: HashSet<LayerType>,
}

impl ClientLayerSet {
    pub fn new(layers: Vec<LayerType>) -> Self {
        Self {
            layers: layers.into_iter().collect(),
            old_layers: HashSet::new(),
        }
    }

    pub fn set(&mut self, layer: LayerType, visibility: bool) -> bool {
        if visibility {
            self.layers.insert(layer)
        } else {
            self.layers.remove(&layer)
        }
    }

    pub fn contains(&self, layer: &LayerType) -> bool {
        self.layers.contains(layer)
    }

    pub fn toggle(&mut self, layer: LayerType) -> bool {
        if self.layers.contains(&layer) {
            self.layers.remove(&layer)
        } else {
            self.layers.insert(layer)
        }
    }

    pub fn clear(&mut self) {
        self.layers.clear();
    }

    pub(crate) fn update(&mut self) {
        self.old_layers = self.layers.clone();
    }

    pub fn added(&self) -> impl Iterator<Item = &LayerType> {
        self.layers.difference(&self.old_layers)
    }

    pub fn removed(&self) -> impl Iterator<Item = &LayerType> {
        self.old_layers.difference(&self.layers)
    }
}

fn update_old_client_layer_set(mut client_layer_set: Query<&mut ClientLayerSet>) {
    for mut client_layer_set in client_layer_set.iter_mut() {
        client_layer_set.update();
    }
}
