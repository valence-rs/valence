use rand::seq::SliceRandom;
use rand::Rng;
use valence::client::despawn_disconnected_clients;
use valence::entity::active_status_effects::{ActiveStatusEffect, ActiveStatusEffects};
use valence::log::LogPlugin;
use valence::network::ConnectionMode;
use valence::prelude::*;
use valence::status_effects::{AttributeModifier, StatusEffect};
use valence_server::entity::attributes::{EntityAttribute, EntityAttributes};
use valence_server::entity::entity::Flags;
use valence_server::entity::living::{Absorption, Health};
use valence_server::status_effect::{StatusEffectAdded, StatusEffectRemoved};

const SPAWN_Y: i32 = 64;

// Notes: Some potion effects are implemented by the client (i.e. we don't need
// to send any more packets than just telling the client about them) and some
// are implemented by the server. The ones implemented by the client are:
// - Jump Boost
// - Night Vision
// - Nausea
// - Blindness
// - Darkness
// - Slow Falling
// - Levitation
// Perhaps also (haven't tested):
// - Dolphin's Grace
// - Conduit Power
//
// There are also a few different potion effects that are implemented by the
// server. Some can be implemented right now, for example:
// - Speed
// - Instant Health
// - Regeneration
// - Absorption
// - Glowing
// - etc. (i.e. the ones with AttributeModifiers, direct health changes or other
//   trivial effects)
//
// Some can't be implemented right now because they require features that aren't
// implemented yet or must be implemented yourself, for example:
// - Water Breathing (requires the ability to breathe underwater)
// - Fire Resistance (requires the ability to not take damage from fire)
// - Hunger (requires the ability to get hungry)
// - Bad Omen (requires the ability to get a raid)
fn main() {
    App::new()
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_systems(Startup, setup)
        .add_systems(
            EventLoopUpdate,
            (
                add_potion_effect,
                handle_status_effect_added,
                handle_status_effect_removed,
            ),
        )
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                handle_status_effect_update,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    biomes: Res<BiomeRegistry>,
    dimensions: Res<DimensionTypeRegistry>,
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

    commands.spawn(layer);
}

#[allow(clippy::type_complexity)]
fn init_clients(
    mut clients: Query<
        (
            &mut Client,
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
        mut client,
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
        pos.set([0.0, SPAWN_Y as f64 + 1.0, 0.0]);
        *game_mode = GameMode::Survival;

        client.send_chat_message("Welcome to the potions example.".bold());
        client.send_chat_message("Sneak to apply a random potion effect.".into_text());
        client.send_chat_message("Note: Some potion effects are not implemented yet.".into_text());
    }
}

pub fn add_potion_effect(
    mut clients: Query<&mut ActiveStatusEffects>,
    mut events: EventReader<SneakEvent>,
) {
    let mut rng = rand::thread_rng();
    for event in events.read() {
        if event.state == SneakState::Start {
            if let Ok(mut status) = clients.get_mut(event.client) {
                status.apply(
                    ActiveStatusEffect::from_effect(*StatusEffect::ALL.choose(&mut rng).unwrap())
                        .with_duration(rng.gen_range(10..1000))
                        .with_amplifier(rng.gen_range(0..5)),
                );
            }
        }
    }
}

fn adjust_modifier_amount(amplifier: u8, amount: f64) -> f64 {
    amount * (amplifier + 1) as f64
}

fn apply_potion_attribute(
    attributes: &mut Mut<EntityAttributes>,
    health: &mut Option<Mut<Health>>,
    amplifier: u8,
    attr: AttributeModifier,
) {
    attributes.remove_modifier(attr.attribute, attr.uuid);

    let amount = adjust_modifier_amount(amplifier, attr.base_value);

    attributes.set_modifier(attr.attribute, attr.uuid, amount, attr.operation);

    // not quite how vanilla does it, but it's close enough
    if attr.attribute == EntityAttribute::GenericMaxHealth {
        if let Some(ref mut health) = health {
            health.0 = health.0.min(
                attributes
                    .get_compute_value(EntityAttribute::GenericMaxHealth)
                    .unwrap_or(0.0) as f32,
            );
        }
    }
}

fn remove_potion_attribute(
    attributes: &mut Mut<EntityAttributes>,
    health: &mut Option<Mut<Health>>,
    attr: AttributeModifier,
) {
    attributes.remove_modifier(attr.attribute, attr.uuid);

    if attr.attribute == EntityAttribute::GenericMaxHealth {
        if let Some(ref mut health) = health {
            health.0 = health.0.min(
                attributes
                    .get_compute_value(EntityAttribute::GenericMaxHealth)
                    .unwrap_or(0.0) as f32,
            );
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn handle_status_effect_added(
    mut clients: Query<(
        &ActiveStatusEffects,
        &mut EntityAttributes,
        Option<&mut Health>,
        Option<&mut Absorption>,
        &mut Flags,
    )>,
    mut events: EventReader<StatusEffectAdded>,
) {
    for event in events.read() {
        if let Ok((status, mut attributes, mut health, absorption, mut flags)) =
            clients.get_mut(event.entity)
        {
            let effect = status.get_current_effect(event.status_effect).unwrap();

            match event.status_effect {
                StatusEffect::Absorption => {
                    // not quite how vanilla does it. if you want to do it the vanilla way, you'll
                    // need to keep track of the previous absorption value and subtract that from
                    // the new value (because you can take damage while having absorption)
                    if let Some(mut absorption) = absorption {
                        absorption.0 += (effect.amplifier() + 1) as f32 * 4.0;
                    }
                }
                StatusEffect::InstantHealth => {
                    if let Some(mut health) = health {
                        health.0 += (4 << effect.amplifier().min(31)) as f32;
                    }
                }
                StatusEffect::InstantDamage => {
                    if let Some(mut health) = health {
                        health.0 -= (6 << effect.amplifier().min(31)) as f32;
                    }
                }
                StatusEffect::Glowing => {
                    flags.set_glowing(true);
                }
                StatusEffect::Invisibility => {
                    flags.set_invisible(true);
                }
                status => {
                    for attr in status.attribute_modifiers() {
                        apply_potion_attribute(
                            &mut attributes,
                            &mut health,
                            effect.amplifier(),
                            attr,
                        );
                    }
                }
            }
        }
    }
}

pub fn handle_status_effect_removed(
    mut clients: Query<(
        &mut EntityAttributes,
        Option<&mut Health>,
        Option<&mut Absorption>,
        &mut Flags,
    )>,
    mut events: EventReader<StatusEffectRemoved>,
) {
    for event in events.read() {
        if let Ok((mut attributes, mut health, absorption, mut flags)) =
            clients.get_mut(event.entity)
        {
            let effect = &event.status_effect;
            match effect.status_effect() {
                StatusEffect::Absorption => {
                    if let Some(mut absorption) = absorption {
                        absorption.0 -= (effect.amplifier() + 1) as f32 * 4.0;
                    }
                }
                StatusEffect::Glowing => {
                    flags.set_glowing(false);
                }
                StatusEffect::Invisibility => {
                    flags.set_invisible(false);
                }
                status => {
                    for attr in status.attribute_modifiers() {
                        remove_potion_attribute(&mut attributes, &mut health, attr);
                    }
                }
            }
        }
    }
}

pub fn handle_status_effect_update(
    mut clients: Query<(&ActiveStatusEffects, &EntityAttributes, Option<&mut Health>)>,
) {
    for (status, attributes, mut health) in &mut clients.iter_mut() {
        for effect in status.get_current_effects() {
            match effect.status_effect() {
                StatusEffect::Regeneration => {
                    let i = 50 >> effect.amplifier().min(31) as u32;

                    if i == 0 || effect.active_ticks() % i == 0 {
                        if let Some(ref mut health) = health {
                            health.0 = (health.0 + 1.0).min(
                                attributes
                                    .get_compute_value(EntityAttribute::GenericMaxHealth)
                                    .unwrap_or(0.0) as f32,
                            );
                        }
                    }
                }
                StatusEffect::Poison => {
                    let i = 25 >> effect.amplifier().min(31) as u32;

                    if i == 0 || effect.active_ticks() % i == 0 {
                        if let Some(ref mut health) = health {
                            health.0 = (health.0 - 1.0).max(1.0);
                        }
                    }
                }
                StatusEffect::Wither => {
                    let i = 40 >> effect.amplifier().min(31) as u32;

                    if i == 0 || effect.active_ticks() % i == 0 {
                        if let Some(ref mut health) = health {
                            health.0 = (health.0 - 1.0).max(0.0);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
