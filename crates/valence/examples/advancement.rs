use valence::prelude::*;
use valence_advancement::bevy_hierarchy::{BuildChildren, Children, Parent};
use valence_advancement::ForceTabUpdate;

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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_systems((init_clients, init_advancements, sneak, tab_change))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Query<&DimensionType>,
    biomes: Query<&Biome>,
) {
    let mut instance = Instance::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            instance.insert_chunk([x, z], Chunk::default());
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
    mut commands: Commands,
    mut clients: Query<(Entity, &mut Location, &mut Position, &mut GameMode), Added<Client>>,
    instances: Query<Entity, With<Instance>>,
) {
    for (client, mut loc, mut pos, mut game_mode) in &mut clients {
        loc.0 = instances.single();
        pos.set([0.5, 65.0, 0.5]);
        *game_mode = GameMode::Creative;
        commands
            .entity(client)
            .insert((RootCriteriaDone(false), TabChangeCount(0)));
    }
}

fn init_advancements(
    mut clients: Query<&mut AdvancementClientUpdate, Added<AdvancementClientUpdate>>,
    root_advancement_query: Query<Entity, (Without<Parent>, With<Advancement>)>,
    children_query: Query<&Children>,
    advancement_check_query: Query<(), With<Advancement>>,
) {
    for mut advancement_client_update in clients.iter_mut() {
        for root_advancement in root_advancement_query.iter() {
            advancement_client_update.send_advancements(
                root_advancement,
                &children_query,
                &advancement_check_query,
            );
        }
    }
}

fn sneak(
    mut sneaking: EventReader<Sneaking>,
    mut client: Query<(&mut AdvancementClientUpdate, &mut RootCriteriaDone)>,
    root_criteria: Query<Entity, With<RootCriteria>>,
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
    }
}

fn tab_change(
    mut tab_change: EventReader<AdvancementTabChange>,
    mut client: Query<(&mut AdvancementClientUpdate, &mut TabChangeCount)>,
    root2_criteria: Query<Entity, With<Root2Criteria>>,
    root: Query<Entity, With<RootAdvancement>>,
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
    }
}
