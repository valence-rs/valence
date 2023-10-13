use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use valence_entity::active_status_effects::{ActiveStatusEffect, ActiveStatusEffects};
use valence_entity::entity::Flags;
use valence_entity::living::{Absorption, PotionSwirlsAmbient, PotionSwirlsColor};
use valence_protocol::packets::play::{
    entity_status_effect_s2c, EntityStatusEffectS2c, RemoveEntityStatusEffectS2c,
};
use valence_protocol::status_effects::StatusEffect;
use valence_protocol::{VarInt, WritePacket};

use crate::client::Client;
use crate::EventLoopPostUpdate;

pub struct StatusEffectPlugin;

impl Plugin for StatusEffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            EventLoopPostUpdate,
            (
                add_status_effects,
                update_active_status_effects,
                add_status_effects, // only expired effects should be here
            ),
        );
    }
}

fn update_active_status_effects(mut query: Query<&mut ActiveStatusEffects>) {
    for mut active_status_effects in query.iter_mut() {
        active_status_effects.increment_active_ticks();

        /* TODO: The following things require to occasionally modify
         * entity stuff:
         * - regeneration
         * - poison
         * - wither
         */
    }
}

fn create_packet(effect: &ActiveStatusEffect) -> EntityStatusEffectS2c {
    EntityStatusEffectS2c {
        // everywhere else in the codebase, this is the player's entity id
        // will probably need to change this later
        entity_id: VarInt(0),
        effect_id: VarInt(effect.status_effect().to_raw() as i32),
        amplifier: effect.amplifier(),
        duration: VarInt(effect.remaining_duration().unwrap_or(-1)),
        flags: entity_status_effect_s2c::Flags::new()
            .with_is_ambient(effect.ambient())
            .with_show_particles(effect.show_particles())
            .with_show_icon(effect.show_icon()),
        factor_codec: None,
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
struct StatusEffectQuery {
    active_effects: &'static mut ActiveStatusEffects,
    client: Option<&'static mut Client>,
    entity_flags: Option<&'static mut Flags>,
    swirl_color: Option<&'static mut PotionSwirlsColor>,
    swirl_ambient: Option<&'static mut PotionSwirlsAmbient>,
    absorption: Option<&'static mut Absorption>,
}

fn add_status_effects(mut query: Query<StatusEffectQuery>) {
    for mut query in query.iter_mut() {
        let updated = query.active_effects.apply_changes();

        if updated.is_empty() {
            continue;
        }

        set_swirl(
            &query.active_effects,
            &mut query.swirl_color,
            &mut query.swirl_ambient,
        );

        for (status_effect, previous_effect) in updated {
            update_status_effect(&mut query, status_effect, previous_effect);
        }
    }
}

fn update_status_effect(
    query: &mut StatusEffectQueryItem,
    status_effect: StatusEffect,
    previous_effect: Option<ActiveStatusEffect>,
) {
    let current_effect = query.active_effects.get_current_effect(status_effect);

    if let Some(ref mut client) = query.client {
        if let Some(updated_effect) = current_effect {
            client.write_packet(&create_packet(updated_effect));
        } else {
            client.write_packet(&RemoveEntityStatusEffectS2c {
                entity_id: VarInt(0),
                effect_id: VarInt(status_effect.to_raw() as i32),
            });
        }
    }

    if let Some(ref mut entity_flags) = query.entity_flags {
        set_entity_flags(status_effect, entity_flags, current_effect.is_some());
    }

    if status_effect == StatusEffect::Absorption {
        if let Some(ref mut absorption) = query.absorption {
            if let Some(prev) = previous_effect {
                absorption.0 -= (prev.amplifier() + 1) as f32 * 4.0;
            }

            if let Some(updated_effect) = current_effect {
                absorption.0 += (updated_effect.amplifier() + 1) as f32 * 4.0;
            }

            if absorption.0 < 0.0 {
                absorption.0 = 0.0;
            }
        }
    }

    // TODO: More stuff such as instant health, instant damage, etc.

    /* TODO: These things require to modify entity attributes:
     * - speed
     * - slowness
     * - haste
     * - mining fatigue
     * - strength
     * - weakness
     * - luck
     * - unluck
     *
     * Entity attributes are not implemented in Valence yet. See
     * #555.
     */
}

fn set_entity_flags(status_effect: StatusEffect, entity_flags: &mut Flags, state: bool) {
    match status_effect {
        StatusEffect::Glowing => {
            entity_flags.set_glowing(state);
        }
        StatusEffect::Invisibility => {
            entity_flags.set_invisible(state);
        }
        _ => {}
    }
}

fn set_swirl(
    active_status_effects: &ActiveStatusEffects,
    swirl_color: &mut Option<Mut<'_, PotionSwirlsColor>>,
    swirl_ambient: &mut Option<Mut<'_, PotionSwirlsAmbient>>,
) {
    if let Some(ref mut swirl_ambient) = swirl_ambient {
        swirl_ambient.0 = active_status_effects
            .get_current_effects()
            .iter()
            .any(|effect| effect.ambient());
    }

    if let Some(ref mut swirl_color) = swirl_color {
        swirl_color.0 = get_color(active_status_effects);
    }
}

/// Ctrl+C Ctrl+V from net.minecraft.potion.PotionUtil#getColor
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
        let l = (status_effect_instance.amplifier() + 1) as f32;
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
