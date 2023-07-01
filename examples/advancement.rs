use std::collections::HashMap;

use valence::advancement::bevy_hierarchy::{BuildChildren, Children, Parent};
use valence::advancement::ForceTabUpdate;
use valence::prelude::*;

#[derive(Component)]
struct RootCriteria;

#[derive(Component)]
struct Root2Criteria;

#[derive(Component)]
struct RootAdvancement;

#[derive(Component)]
struct RootCriteriaDone(bool);

#[derive(Component)]
struct TabChangeCount(u8);

#[derive(Resource, Default)]
struct ClientSave(HashMap<Uuid, (bool, u8)>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<ClientSave>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                load_clients,
                apply_deferred.after(load_clients).before(init_advancements),
                init_clients,
                init_advancements,
                sneak,
                tab_change,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            instance.set_block([x, 64, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(instance);

    let root_criteria = commands
        .spawn((
            AdvancementCriteria::new(ident!("custom:root_criteria").into()),
            RootCriteria,
        ))
        .id();

    let root_advancement = commands
        .spawn((
            AdvancementBundle {
                advancement: Advancement::new(ident!("custom:root").into()),
                requirements: AdvancementRequirements(vec![vec![root_criteria]]),
                cached_bytes: Default::default(),
            },
            AdvancementDisplay {
                title: "Root".into(),
                description: "Toggles when you sneak".into(),
                icon: Some(ItemStack::new(ItemKind::Stone, 1, None)),
                frame_type: AdvancementFrameType::Task,
                show_toast: true,
                hidden: false,
                background_texture: Some(ident!("textures/block/stone.png").into()),
                x_coord: 0.0,
                y_coord: 0.0,
            },
            RootAdvancement,
        ))
        .add_child(root_criteria)
        .id();

    commands
        .spawn((
            AdvancementBundle {
                advancement: Advancement::new(ident!("custom:first").into()),
                requirements: AdvancementRequirements::default(),
                cached_bytes: Default::default(),
            },
            AdvancementDisplay {
                title: "First".into(),
                description: "First advancement".into(),
                icon: Some(ItemStack::new(ItemKind::OakWood, 1, None)),
                frame_type: AdvancementFrameType::Task,
                show_toast: false,
                hidden: false,
                background_texture: None,
                x_coord: 1.0,
                y_coord: -0.5,
            },
        ))
        .set_parent(root_advancement);

    commands
        .spawn((
            AdvancementBundle {
                advancement: Advancement::new(ident!("custom:second").into()),
                requirements: AdvancementRequirements::default(),
                cached_bytes: Default::default(),
            },
            AdvancementDisplay {
                title: "Second".into(),
                description: "Second advancement".into(),
                icon: Some(ItemStack::new(ItemKind::AcaciaWood, 1, None)),
                frame_type: AdvancementFrameType::Task,
                show_toast: false,
                hidden: false,
                background_texture: None,
                x_coord: 1.0,
                y_coord: 0.5,
            },
        ))
        .set_parent(root_advancement);

    let root2_criteria = commands
        .spawn((
            AdvancementCriteria::new(ident!("custom:root2_criteria").into()),
            Root2Criteria,
        ))
        .id();

    commands
        .spawn((
            AdvancementBundle {
                advancement: Advancement::new(ident!("custom:root2").into()),
                requirements: AdvancementRequirements(vec![vec![root2_criteria]]),
                cached_bytes: Default::default(),
            },
            AdvancementDisplay {
                title: "Root2".into(),
                description: "Go to this tab 5 times to earn this advancement".into(),
                icon: Some(ItemStack::new(ItemKind::IronSword, 1, None)),
                frame_type: AdvancementFrameType::Challenge,
                show_toast: false,
                hidden: false,
                background_texture: Some(Ident::new("textures/block/andesite.png").unwrap()),
                x_coord: 0.0,
                y_coord: 0.0,
            },
        ))
        .add_child(root2_criteria);
}

fn init_clients(
    mut clients: Query<(&mut Location, &mut Position, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (mut loc, mut pos, mut game_mode) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.5, 65.0, 0.5]);
        *game_mode = GameMode::Creative;
    }
}

fn load_clients(
    mut commands: Commands,
    clients: Query<(Entity, &UniqueId), Added<Client>>,
    mut client_save: ResMut<ClientSave>,
) {
    for (client, uuid) in clients.iter() {
        let (root_criteria_done, tab_change_count) =
            client_save.0.entry(uuid.0).or_insert((false, 0));

        commands.entity(client).insert((
            RootCriteriaDone(*root_criteria_done),
            TabChangeCount(*tab_change_count),
        ));
    }
}

fn init_advancements(
    mut clients: Query<
        (
            &mut AdvancementClientUpdate,
            &RootCriteriaDone,
            &TabChangeCount,
        ),
        Added<AdvancementClientUpdate>,
    >,
    root_advancement_query: Query<Entity, (Without<Parent>, With<Advancement>)>,
    children_query: Query<&Children>,
    advancement_check_query: Query<(), With<Advancement>>,
    root2_criteria: Query<Entity, With<Root2Criteria>>,
    root_criteria: Query<Entity, With<RootCriteria>>,
) {
    let root_c = root_criteria.single();
    let root2_c = root2_criteria.single();
    for (mut advancement_client_update, root_criteria, tab_change) in clients.iter_mut() {
        for root_advancement in root_advancement_query.iter() {
            advancement_client_update.send_advancements(
                root_advancement,
                &children_query,
                &advancement_check_query,
            );
            if root_criteria.0 {
                advancement_client_update.criteria_done(root_c);
            }
            if tab_change.0 > 5 {
                advancement_client_update.criteria_done(root2_c);
            }
        }
    }
}

fn sneak(
    mut sneaking: EventReader<SneakEvent>,
    mut client: Query<(&mut AdvancementClientUpdate, &mut RootCriteriaDone)>,
    root_criteria: Query<Entity, With<RootCriteria>>,
    client_uuid: Query<&UniqueId>,
    mut client_save: ResMut<ClientSave>,
) {
    let root_criteria = root_criteria.single();
    for sneaking in sneaking.iter() {
        if sneaking.state == SneakState::Stop {
            continue;
        }
        let Ok((mut advancement_client_update, mut root_criteria_done)) = client.get_mut(sneaking.client) else { continue; };
        root_criteria_done.0 = !root_criteria_done.0;
        match root_criteria_done.0 {
            true => advancement_client_update.criteria_done(root_criteria),
            false => advancement_client_update.criteria_undone(root_criteria),
        }
        client_save
            .0
            .get_mut(&client_uuid.get(sneaking.client).unwrap().0)
            .unwrap()
            .0 = root_criteria_done.0;
    }
}

fn tab_change(
    mut tab_change: EventReader<AdvancementTabChangeEvent>,
    mut client: Query<(&mut AdvancementClientUpdate, &mut TabChangeCount)>,
    root2_criteria: Query<Entity, With<Root2Criteria>>,
    root: Query<Entity, With<RootAdvancement>>,
    client_uuid: Query<&UniqueId>,
    mut client_save: ResMut<ClientSave>,
) {
    let root2_criteria = root2_criteria.single();
    let root = root.single();
    for tab_change in tab_change.iter() {
        let Ok((mut advancement_client_update, mut tab_change_count)) = client.get_mut(tab_change.client) else { continue; };
        if let Some(ref opened) = tab_change.opened_tab {
            if opened.as_str() == "custom:root2" {
                tab_change_count.0 += 1;
            } else {
                continue;
            }
        } else {
            continue;
        }
        if tab_change_count.0 == 5 {
            advancement_client_update.criteria_done(root2_criteria);
        } else if tab_change_count.0 >= 10 {
            advancement_client_update.force_tab_update = ForceTabUpdate::Spec(root);
        }
        client_save
            .0
            .get_mut(&client_uuid.get(tab_change.client).unwrap().0)
            .unwrap()
            .1 = tab_change_count.0;
    }
}
