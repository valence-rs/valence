use std::collections::HashSet;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_core::chunk_pos::ChunkView;
use valence_core::layer::{Layer, LayerType};
use valence_entity::{OldPosition, Position};

use crate::{
    read_data_in_old_view, Client, EntityInitQuery, EntityRemoveBuf, ViewDistance,
};

pub(super) fn build(app: &mut App) {
    app.add_system(update_layer_view.before(read_data_in_old_view));
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
    pub fn new(layers: impl Iterator<Item = LayerType>) -> Self {
        let mut set = Self {
            layers: layers.collect(),
            old_layers: HashSet::new(),
        };
        set.update();
        set
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

fn update_layer_view(
    mut clients: Query<
        (
            &mut Client,
            &mut EntityRemoveBuf,
            &Position,
            &ViewDistance,
            &mut ClientLayerSet,
        ),
        Changed<ClientLayerSet>,
    >,
    entities: Query<(EntityInitQuery, &OldPosition, &Layer)>,
) {
    clients.par_iter_mut().for_each_mut(
        |(mut client, mut remove_buf, pos, view_dist, mut client_layer_set)| {
            let view = ChunkView::new(pos.chunk_pos(), view_dist.0);

            // Send entity spawn packets for entities that are in an entered layer and
            // already in the client's view.
            for (entity_init, &old_pos, layer) in entities.iter() {
                if view.contains(old_pos.chunk_pos())
                    && client_layer_set.added().any(|l| l == &layer.0)
                {
                    entity_init.write_init_packets(old_pos.get(), &mut client.enc);
                }
            }

            // Send entity despawn packets for entities that are in an exited layer and
            // still in the client's view.
            for (entity_init, &old_pos, layer) in entities.iter() {
                if view.contains(old_pos.chunk_pos())
                    && client_layer_set.removed().any(|l| l == &layer.0)
                {
                    remove_buf.push(entity_init.entity_id.get());
                }
            }

            client_layer_set.update();
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let layers = vec![0, 1, 2];
        let client_layer_set = ClientLayerSet::new(layers.iter().cloned());

        assert!(client_layer_set.contains(&0));
        assert!(client_layer_set.contains(&1));
        assert!(client_layer_set.contains(&2));
    }

    #[test]
    fn test_set() {
        let mut client_layer_set = ClientLayerSet::new(vec![0, 1, 2].into_iter());

        assert!(client_layer_set.contains(&0));
        assert!(client_layer_set.contains(&1));
        assert!(client_layer_set.contains(&2));

        client_layer_set.set(1, false);

        assert!(client_layer_set.contains(&0));
        assert!(!client_layer_set.contains(&1));
        assert!(client_layer_set.contains(&2));
    }

    #[test]
    fn test_toggle() {
        let mut client_layer_set = ClientLayerSet::new(vec![0, 1, 2].into_iter());

        assert!(client_layer_set.contains(&0));
        assert!(client_layer_set.contains(&1));
        assert!(client_layer_set.contains(&2));

        client_layer_set.toggle(1);

        assert!(client_layer_set.contains(&0));
        assert!(!client_layer_set.contains(&1));
        assert!(client_layer_set.contains(&2));

        client_layer_set.toggle(1);

        assert!(client_layer_set.contains(&0));
        assert!(client_layer_set.contains(&1));
        assert!(client_layer_set.contains(&2));
    }

    #[test]
    fn test_clear() {
        let mut client_layer_set = ClientLayerSet::new(vec![0, 1, 2].into_iter());

        assert!(client_layer_set.contains(&0));
        assert!(client_layer_set.contains(&1));
        assert!(client_layer_set.contains(&2));

        client_layer_set.clear();

        assert!(!client_layer_set.contains(&0));
        assert!(!client_layer_set.contains(&1));
        assert!(!client_layer_set.contains(&2));
    }

    #[test]
    fn test_added() {
        let mut client_layer_set = ClientLayerSet::new(vec![0, 1, 2].into_iter());

        client_layer_set.set(1, false);
        client_layer_set.set(3, true);

        let added: Vec<&LayerType> = client_layer_set.added().collect();

        assert_eq!(added, vec![&3]);
    }

    #[test]
    fn test_removed() {
        let mut client_layer_set = ClientLayerSet::new(vec![0, 1, 2].into_iter());

        client_layer_set.set(1, false);
        client_layer_set.set(3, true);

        let removed: Vec<&LayerType> = client_layer_set.removed().collect();

        assert_eq!(removed, vec![&1]);
    }
}
