use valence::entity::player::PlayerEntityBundle;
use valence::player_list::{DisplayName, PlayerListEntryBundle};
use valence::prelude::*;
use valence::property::Property;
use valence::text::IntoText;

const SPAWN_Y: i32 = 64;

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, despawn_disconnected_clients))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    let layer_id = commands.spawn(layer).id();

    let npc_id = UniqueId::default();
    commands.spawn(PlayerEntityBundle {
        layer: EntityLayerId(layer_id),
        uuid: npc_id,
        position: Position::new((0.0, SPAWN_Y as f64 + 1.0, 6.0)),
        look: Look::new(180.0, 0.0),
        head_yaw: HeadYaw(180.0),
        ..Default::default()
    });

    let mut props = Properties::default();
    props.0.push(Property {
        name: "textures".to_owned(),
        value:
            "ewogICJ0aW1lc3RhbXAiIDogMTY5MTcwNjU3MzE1NiwKICAicHJvZmlsZUlkIiA6ICJlODgyNzRlYjNmNTE0ZDYwYmMxYWQ5NTQ4MTIxODMwMyIsCiAgInByb2ZpbGVOYW1lIiA6ICJBbmltYWxUaGVHYW1lciIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS8xZGUyYzgzZjhmNGZiMzgwNjlmNTVmNTJlNGY4ZWU1ZjA4NjcyMjllYWQ5MWI3ZTc5ZGVmNzU0YjcwZWE5NDMzIiwKICAgICAgIm1ldGFkYXRhIiA6IHsKICAgICAgICAibW9kZWwiIDogInNsaW0iCiAgICAgIH0KICAgIH0KICB9Cn0="
            .to_owned(),
        signature:
            "k/g8JTYB0A5O+h8+XSdw3QFEVHnzomDsGl6eubV/sE396yAL7E4qCT24r3Uv88YYforuET1BXG0GBOewcij3uMajm+mc/P7v+0+C+NSS9g5dpSs2e9MdeGZBgDEr1kTnXzQmayZUvLGitW23GuRDHdVHx76JZpxBk3q0VsjgncNs6UVZwfYNCaUGZZx38bqG5FXGxE0MfFHKiJawKwWRaoAbHjrfsByLipIKUhssUF3pt+HPWbgaOD2rO0EOLBrGzvEnu9oeLPH4tqdlvurjGrdpM4wKCmS3j8K91OBTABciVR9xt0fRnhbL4JoZuLK+iefNXx8nBCVEOm9sNk4pXHNWZvKEkqMb3jvpxuYHsSZPm0IdN+74FEmjHy0sY/7+ZG/h/IUHs4CyrPAtR/rqON6MG8nVVBxUq4kWV+2Xj+U+O02gQUVFqMM77AqArRsPIkeFIgVQ6+WvBZYXuRe1Ryo6qwjmYGc4AeTZTtvafzv8vfAMFfJJmT69nkTTDO5hAtDTUnCd86nNFQ3qijdO9CW7OFDyysb9M0a1O7pQ7Nu10rkNwY+6uTfKoATtT80+RoMzvKwcIAG4cY+PR5jhsKP+sf+AEymovD+cPVnLOuZQ6bAyKW6yjf9Xd0vyirCgNaU1CGmDE1mihGK2kC0fm11RaoDbyKvMcLKAq+OFos0="
            .to_owned().into(),
    });
    commands.spawn(PlayerListEntryBundle {
        uuid: npc_id,
        username: Username("Alice".into()),
        display_name: DisplayName("Alice".color(Color::RED).into()),
        properties: props,
        ..Default::default()
    });
    // npc.insert(npc);
}

fn init_clients(
    mut clients: Query<
        (
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.0, 65.0, 0.0]);
        *game_mode = GameMode::Creative;
    }
}
