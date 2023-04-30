use valence::prelude::*;
use valence_advancement::bevy_hierarchy::{BuildChildren, Children, Parent};

#[derive(Component)]
struct RootCriteria;

#[derive(Component)]
struct RootCriteriaDone(bool);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(CoreSettings {
            compression_threshold: None,
            ..Default::default()
        })
        .add_startup_system(setup)
        .add_systems((init_clients, init_advancements, sneak))
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
            AdvancementCriteria::new(Ident::new("custom:root_criteria").unwrap()),
            RootCriteria,
        ))
        .id();

    let root_advancement = commands
        .spawn((
            AdvancementBundle::new(Ident::new("custom:root").unwrap()),
            AdvancementDisplay {
                title: "Root".into(),
                description: "Toggles when you sneak".into(),
                icon: Some(ItemStack::new(ItemKind::Stone, 1, None)),
                frame_type: AdvancementFrameType::Task,
                show_toast: false,
                hidden: false,
                background_texture: Some(Ident::new("textures/block/stone.png").unwrap()),
                x_coord: 0.0,
                y_coord: 0.0,
            },
            AdvancementRequirements(vec![vec![root_criteria]]),
        ))
        .add_child(root_criteria)
        .id();

    commands
        .spawn((
            AdvancementBundle::new(Ident::new("custom:first").unwrap()),
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
            AdvancementRequirements(vec![]),
        ))
        .set_parent(root_advancement);

    commands
        .spawn((
            AdvancementBundle::new(Ident::new("custom:second").unwrap()),
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
            AdvancementRequirements(vec![]),
        ))
        .set_parent(root_advancement);
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
        commands.entity(client).insert(RootCriteriaDone(false));
    }
}

fn init_advancements(
    mut clients: Query<&mut AdvancementClientUpdate, Added<AdvancementClientUpdate>>,
    root_advancement_query: Query<Entity, (Without<Parent>, With<Advancement>)>,
    children_query: Query<&Children>,
    advancement_check_query: Query<(), With<Advancement>>,
) {
    let root_advancement = root_advancement_query.get_single().unwrap();
    for mut advancement_client_update in clients.iter_mut() {
        advancement_client_update.send_advancements(
            root_advancement,
            &children_query,
            &advancement_check_query,
        );
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
