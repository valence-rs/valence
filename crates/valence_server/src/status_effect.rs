use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_entity::active_status_effects::{ActiveStatusEffect, ActiveStatusEffects};
use valence_entity::entity::Flags;
use valence_entity::living::{PotionSwirlsAmbient, PotionSwirlsColor};
use valence_entity::status_effects::StatusEffect;
use valence_protocol::packets::play::{
    entity_status_effect_s2c, EntityStatusEffectS2c, RemoveEntityStatusEffectS2c,
};
use valence_protocol::{VarInt, WritePacket};

use crate::client::Client;
use crate::EventLoopPostUpdate;

pub struct StatusEffectPlugin;

impl Plugin for StatusEffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            EventLoopPostUpdate,
            (
                remove_expired_status_effects,
                update_active_status_effects,
                add_status_effects,
            ),
        );
    }
}

fn update_active_status_effects(mut query: Query<&mut ActiveStatusEffects>) {
    for mut active_status_effects in query.iter_mut() {
        for effect in active_status_effects.active_effects_mut().iter_mut() {
            effect.decrement_duration();
            /* TODO: The following things require to occasionally modify
             * entity stuff:
             * - regeneration
             * - poison
             * - wither
             */
        }
    }
}

fn create_packet(effect: &ActiveStatusEffect) -> EntityStatusEffectS2c {
    EntityStatusEffectS2c {
        entity_id: VarInt(0),
        effect_id: VarInt(effect.status_effect().to_raw() as i32),
        amplifier: effect.amplifier(),
        duration: VarInt(effect.duration().unwrap_or(-1)),
        flags: entity_status_effect_s2c::Flags::new()
            .with_is_ambient(effect.ambient())
            .with_show_particles(effect.show_particles())
            .with_show_icon(effect.show_icon()),
        factor_codec: None,
    }
}

fn add_status_effects(
    mut query: Query<(
        &mut ActiveStatusEffects,
        Option<&mut Client>,
        Option<&mut Flags>,
        Option<&mut PotionSwirlsColor>,
        Option<&mut PotionSwirlsAmbient>,
    )>,
) {
    for (
        mut active_status_effects,
        mut client,
        mut entity_flags,
        mut swirl_color,
        mut swirl_ambient,
    ) in query.iter_mut()
    {
        for new_effect in active_status_effects.move_new_to_active() {
            let status_effect = new_effect.status_effect();

            if let Some(ref mut client) = client {
                client.write_packet(&create_packet(new_effect));
            }

            if let Some(ref mut entity_flags) = entity_flags {
                set_entity_flags(status_effect, entity_flags, true);
            }

            if let Some(ref mut swirl_color) = swirl_color {
                // TODO: Mix colors.
                // See net.minecraft.potion.PotionUtil#getColor lines 90-115
                swirl_color.0 = status_effect.color() as i32; // TODO: `as i32`?
                                                              // ???
            }

            if let Some(ref mut swirl_ambient) = swirl_ambient {
                swirl_ambient.0 = new_effect.ambient();
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
             */
        }
    }
}

fn remove_expired_status_effects(
    mut query: Query<(
        &mut ActiveStatusEffects,
        Option<&mut Client>,
        Option<&mut Flags>,
    )>,
) {
    for (mut active_status_effects, mut client, mut entity_flags) in query.iter_mut() {
        for effect in active_status_effects.active_effects_mut() {
            if effect.expired() {
                let status_effect = effect.status_effect();

                if let Some(ref mut client) = client {
                    client.write_packet(&RemoveEntityStatusEffectS2c {
                        entity_id: VarInt(0),
                        effect_id: VarInt(status_effect.to_raw() as i32),
                    });
                }

                if let Some(ref mut entity_flags) = entity_flags {
                    set_entity_flags(status_effect, entity_flags, false);
                }
            }
        }

        active_status_effects.remove_expired();
    }
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
