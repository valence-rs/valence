use bevy_ecs::prelude::*;
use bytes::Bytes;
use serde::Deserialize;
use valence_core::ident::Ident;
use valence_core::protocol::packet::synchronize_tags::*;
use valence_core::protocol::Encode;

#[derive(Debug, Resource)]
pub struct TagsRegistry {
    pub registries: Vec<Registry>,
    cached_packet: Bytes,
}

#[derive(Debug, Deserialize)]
pub struct Registry {
    #[serde(rename = "registry")]
    pub kind: Ident<String>,
    pub tags: Vec<TagEntry>,
}

#[derive(Debug, Deserialize)]
pub struct TagEntry {
    pub name: Ident<String>,
    #[serde(rename = "values")]
    pub entries: Vec<u64>,
}

impl TagsRegistry {
    pub(crate) fn build_synchronize_tags<'a>(&'a self) -> SynchronizeTagsS2c<'a> {
        SynchronizeTagsS2c {
            tags: self
                .registries
                .iter()
                .map(|registry| TagGroup {
                    kind: registry.kind.as_str_ident().into(),
                    tags: registry
                        .tags
                        .iter()
                        .map(|tag| Tag {
                            name: tag.name.as_str_ident().into(),
                            entries: tag.entries.clone(),
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn sync_tags_packet(&self) -> &Bytes {
        &self.cached_packet
    }
}

pub fn init_tags_registry(mut tags: ResMut<TagsRegistry>) {
    let registries =
        serde_json::from_str::<Vec<Registry>>(include_str!("../../../extracted/tags.json"))
            .expect("tags.json is invalid");
    tags.registries = registries;
}

pub fn cache_tags_packet(tags: ResMut<TagsRegistry>) {
    if tags.is_changed() {
        let tags = tags.into_inner();
        let packet = tags.build_synchronize_tags();
        let mut bytes = vec![];
        packet
            .encode(&mut bytes)
            .expect("failed to encode tags packet");
        tags.cached_packet = bytes.into();
    }
}

impl Default for TagsRegistry {
    fn default() -> Self {
        Self {
            registries: vec![],
            cached_packet: Bytes::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPlugin;

    #[test]
    fn smoke_test() {
        let mut app = bevy_app::App::new();
        app.add_plugin(RegistryPlugin);
        app.update();

        let tags_registry = app.world.get_resource::<TagsRegistry>().unwrap();
        let packet = tags_registry.build_synchronize_tags();
        assert!(packet.tags.len() > 0);
    }
}
