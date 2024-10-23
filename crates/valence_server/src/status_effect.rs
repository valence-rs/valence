use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryData;
use bevy_ecs::system::SystemState;
use valence_entity::active_status_effects::{ActiveStatusEffect, ActiveStatusEffects};
use valence_entity::entity::Flags;
use valence_entity::living::PotionSwirlsAmbient;
use valence_protocol::packets::play::{
    update_mob_effect_s2c, RemoveMobEffectS2c, UpdateMobEffectS2c,
};
use valence_protocol::status_effects::StatusEffect;
use valence_protocol::{VarInt, WritePacket};

use crate::client::Client;
use crate::EventLoopPostUpdate;

/// Event for when a status effect is added to an entity or the amplifier or
/// duration of an existing status effect is changed.
#[derive(Event, Clone, PartialEq, Eq, Debug)]
pub struct StatusEffectAdded {
    pub entity: Entity,
    pub status_effect: StatusEffect,
}

/// Event for when a status effect is removed from an entity.
#[derive(Event, Clone, PartialEq, Eq, Debug)]
pub struct StatusEffectRemoved {
    pub entity: Entity,
    pub status_effect: ActiveStatusEffect,
}

pub struct StatusEffectPlugin;

impl Plugin for StatusEffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StatusEffectAdded>()
            .add_event::<StatusEffectRemoved>()
            .add_systems(
                EventLoopPostUpdate,
                (
                    add_status_effects,
                    update_active_status_effects,
                    add_status_effects,
                ),
            );
    }
}

fn update_active_status_effects(
    world: &mut World,
    state: &mut SystemState<Query<&mut ActiveStatusEffects>>,
) {
    let mut query = state.get_mut(world);
    for mut active_status_effects in &mut query {
        active_status_effects.increment_active_ticks();
    }
}

fn create_packet(effect: &ActiveStatusEffect) -> UpdateMobEffectS2c {
    UpdateMobEffectS2c {
        entity_id: VarInt(0), // We reserve ID 0 for clients.
        effect_id: VarInt(i32::from(effect.status_effect().to_raw())),
        amplifier: effect.amplifier(),
        duration: VarInt(effect.remaining_duration().unwrap_or(-1)),
        flags: update_mob_effect_s2c::Flags::new()
            .with_is_ambient(effect.ambient())
            .with_show_particles(effect.show_particles())
            .with_show_icon(effect.show_icon()),
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
struct StatusEffectQuery {
    entity: Entity,
    active_effects: &'static mut ActiveStatusEffects,
    client: Option<&'static mut Client>,
    entity_flags: Option<&'static mut Flags>,
    swirl_ambient: Option<&'static mut PotionSwirlsAmbient>,
}

fn add_status_effects(
    mut query: Query<StatusEffectQuery>,
    mut add_events: EventWriter<StatusEffectAdded>,
    mut remove_events: EventWriter<StatusEffectRemoved>,
) {
    for mut query in &mut query {
        let updated = query.active_effects.apply_changes();

        if updated.is_empty() {
            continue;
        }

        set_swirl(&query.active_effects, &mut query.swirl_ambient);

        for (status_effect, prev) in updated {
            if query.active_effects.has_effect(status_effect) {
                add_events.send(StatusEffectAdded {
                    entity: query.entity,
                    status_effect,
                });
            } else if let Some(prev) = prev {
                remove_events.send(StatusEffectRemoved {
                    entity: query.entity,
                    status_effect: prev,
                });
            } else {
                // this should never happen
                panic!("status effect was removed but was never added");
            }

            update_status_effect(&mut query, status_effect);
        }
    }
}

fn update_status_effect(query: &mut StatusEffectQueryItem, status_effect: StatusEffect) {
    let current_effect = query.active_effects.get_current_effect(status_effect);

    if let Some(ref mut client) = query.client {
        if let Some(updated_effect) = current_effect {
            client.write_packet(&create_packet(updated_effect));
        } else {
            client.write_packet(&RemoveMobEffectS2c {
                entity_id: VarInt(0),
                effect_id: VarInt(i32::from(status_effect.to_raw())),
            });
        }
    }
}

fn set_swirl(
    active_status_effects: &ActiveStatusEffects,
    swirl_ambient: &mut Option<Mut<'_, PotionSwirlsAmbient>>,
) {
    if let Some(ref mut swirl_ambient) = swirl_ambient {
        swirl_ambient.0 = active_status_effects
            .get_current_effects()
            .iter()
            .any(|effect| effect.ambient());
    }
}

/// Used to set the color of the swirls in the potion effect.
///
/// Equivalent to net.minecraft.potion.PotionUtil#getColor
fn get_color(effects: &ActiveStatusEffects) -> i32 {
    if effects.no_effects() {
        // vanilla mc seems to return 0x385dc6 if there are no effects
        // dunno why
        // imma just say to return 0 to remove the swirls
        return 0;
    }

    let effects = effects.get_current_effects();
    let mut f = 0.0;
    let mut g = 0.0;
    let mut h = 0.0;
    let mut j = 0.0;

    for status_effect_instance in effects {
        if !status_effect_instance.show_particles() {
            continue;
        }

        let k = status_effect_instance.status_effect().color();
        let l = f32::from(status_effect_instance.amplifier() + 1);
        f += (l * ((k >> 16) & 0xff) as f32) / 255.0;
        g += (l * ((k >> 8) & 0xff) as f32) / 255.0;
        h += (l * ((k) & 0xff) as f32) / 255.0;
        j += l;
    }

    if j == 0.0 {
        return 0;
    }

    f = f / j * 255.0;
    g = g / j * 255.0;
    h = h / j * 255.0;

    ((f as i32) << 16) | ((g as i32) << 8) | (h as i32)
}
