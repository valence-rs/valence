use bevy_ecs::prelude::*;
use serde::Deserialize;
use valence_core::ident::Ident;
use valence_core::protocol::encode::{PacketWriter, WritePacket};
use valence_core::protocol::packet::synchronize_tags::*;
use valence_core::Server;

#[derive(Debug, Resource)]
pub struct TagsRegistry {
    pub registries: Vec<Registry>,
    cached_packet: Vec<u8>,
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
    pub(crate) fn build_synchronize_tags(&self) -> SynchronizeTagsS2c<'_> {
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

    pub fn sync_tags_packet(&self) -> &Vec<u8> {
        &self.cached_packet
    }
}

pub(crate) fn init_tags_registry(mut tags: ResMut<TagsRegistry>) {
    let registries =
        serde_json::from_str::<Vec<Registry>>(include_str!("../../../extracted/tags.json"))
            .expect("tags.json is invalid");
    tags.registries = registries;
}

pub(crate) fn cache_tags_packet(server: Res<Server>, tags: ResMut<TagsRegistry>) {
    if tags.is_changed() {
        let tags = tags.into_inner();
        let packet = tags.build_synchronize_tags();
        let mut bytes = vec![];
        let mut scratch = vec![];
        let mut writer =
            PacketWriter::new(&mut bytes, server.compression_threshold(), &mut scratch);
        writer.write_packet(&packet);
        tags.cached_packet = bytes;
    }
}

impl Default for TagsRegistry {
    fn default() -> Self {
        Self {
            registries: vec![],
            cached_packet: Vec::new(),
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
        app.insert_resource(Server::default());
        app.update();

        let tags_registry = app.world.get_resource::<TagsRegistry>().unwrap();
        let packet = tags_registry.build_synchronize_tags();
        assert!(!packet.tags.is_empty());
    }
}
