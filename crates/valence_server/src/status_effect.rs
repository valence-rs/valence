use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_entity::EntityId;
use valence_protocol::packets::play::{
    entity_status_effect_s2c, EntityStatusEffectS2c, RemoveEntityStatusEffectS2c,
};
use valence_protocol::{StatusEffect, VarInt, WritePacket};

use crate::client::Client;
use crate::EventLoopPostUpdate;

/// [`Component`] that stores the [`ActiveStatusEffect`]s of an [`Entity`].
#[derive(Component, Default)]
pub struct ActiveStatusEffects {
    active: Vec<ActiveStatusEffect>,
    new: Vec<ActiveStatusEffect>,
}

impl ActiveStatusEffects {
    pub fn add(&mut self, effect: ActiveStatusEffect) {
        self.new.push(effect);
    }
}

/// Represents an active status effect.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ActiveStatusEffect {
    effect: StatusEffect,
    amplifier: u8,
    /// The total duration of the status effect in ticks.
    duration: i32,
    ambient: bool,
    show_particles: bool,
    show_icon: bool,
}

impl ActiveStatusEffect {
    /// Creates a new [`ActiveStatusEffect`].
    pub fn from_effect(effect: StatusEffect) -> Self {
        Self {
            effect,
            amplifier: 0,
            duration: 600,
            ambient: false,
            show_particles: true,
            show_icon: true,
        }
    }

    /// Sets the amplifier of the [`ActiveStatusEffect`].
    pub fn with_amplifier(mut self, amplifier: u8) -> Self {
        self.amplifier = amplifier;
        self
    }

    /// Sets the duration of the [`ActiveStatusEffect`] in ticks.
    pub fn with_duration(mut self, duration: i32) -> Self {
        self.duration = duration;
        self
    }

    /// Sets the duration of the [`ActiveStatusEffect`] in seconds.
    pub fn with_duration_seconds(mut self, duration: f32) -> Self {
        self.duration = (duration * 20.0) as i32;
        self
    }

    /// Sets the duration of the [`ActiveStatusEffect`] to infinite.
    pub fn with_infinite_duration(mut self) -> Self {
        self.duration = -1; // -1 is infinite in vanilla
        self
    }

    /// Sets whether the [`ActiveStatusEffect`] is ambient.
    pub fn with_ambient(mut self, ambient: bool) -> Self {
        self.ambient = ambient;
        self
    }

    /// Sets whether the [`ActiveStatusEffect`] shows particles.
    pub fn with_show_particles(mut self, show_particles: bool) -> Self {
        self.show_particles = show_particles;
        self
    }

    /// Sets whether the [`ActiveStatusEffect`] shows an icon.
    pub fn with_show_icon(mut self, show_icon: bool) -> Self {
        self.show_icon = show_icon;
        self
    }

    /// Decrements the duration of the [`ActiveStatusEffect`] by a tick.
    pub fn decrement_duration(&mut self) {
        if self.duration < 0 {
            return;
        }
        self.duration -= 1;
        if self.duration < 0 {
            self.duration = 0;
        }
    }

    /// Returns the [`StatusEffect`] of the [`ActiveStatusEffect`].
    pub fn status_effect(&self) -> StatusEffect {
        self.effect
    }

    /// Returns the amplifier of the [`ActiveStatusEffect`].
    pub fn amplifier(&self) -> u8 {
        self.amplifier
    }

    /// Returns the remaining duration of the [`ActiveStatusEffect`] in ticks.
    pub fn duration(&self) -> i32 {
        self.duration
    }

    /// Returns whether the [`ActiveStatusEffect`] is ambient.
    pub fn ambient(&self) -> bool {
        self.ambient
    }

    /// Returns whether the [`ActiveStatusEffect`] shows particles.
    pub fn show_particles(&self) -> bool {
        self.show_particles
    }

    /// Returns whether the [`ActiveStatusEffect`] shows an icon.
    pub fn show_icon(&self) -> bool {
        self.show_icon
    }

    /// Returns true if the [`ActiveStatusEffect`] has expired.
    pub fn expired(&self) -> bool {
        self.duration == 0
    }
}

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
        for effect in active_status_effects.active.iter_mut() {
            effect.decrement_duration();
        }
    }
}

fn add_status_effects(
    mut query: Query<(&EntityId, &mut ActiveStatusEffects)>,
    mut clients: Query<(&EntityId, &mut Client)>,
) {
    for (entity_id, mut active_status_effects) in query.iter_mut() {
        let entity_id = entity_id.get();
        for new_effect in &active_status_effects.new {
            for (client_id, mut client) in clients.iter_mut() {
                let client_id = client_id.get();
                if client_id == entity_id {
                    client.write_packet(&EntityStatusEffectS2c {
                        entity_id: VarInt(0),
                        effect_id: VarInt(new_effect.status_effect().to_raw() as i32),
                        amplifier: new_effect.amplifier(),
                        duration: VarInt(new_effect.duration()),
                        flags: entity_status_effect_s2c::Flags::new()
                            .with_is_ambient(new_effect.ambient())
                            .with_show_particles(new_effect.show_particles())
                            .with_show_icon(new_effect.show_icon()),
                        factor_codec: None,
                    });
                } else {
                    // TODO: Something with EntityTrackerUpdateS2c
                }
            }
        }

        let mut old_vec = std::mem::take(&mut active_status_effects.new);
        active_status_effects.active.append(&mut old_vec);
    }
}

fn remove_expired_status_effects(
    mut query: Query<(&EntityId, &mut ActiveStatusEffects)>,
    mut clients: Query<(&EntityId, &mut Client)>,
) {
    for (entity_id, mut active_status_effects) in query.iter_mut() {
        let entity_id = entity_id.get();

        for effect in &active_status_effects.active {
            if effect.expired() {
                for (client_id, mut client) in clients.iter_mut() {
                    let client_id = client_id.get();
                    client.write_packet(&RemoveEntityStatusEffectS2c {
                        entity_id: VarInt(if client_id == entity_id { 0 } else { entity_id }),
                        effect_id: VarInt(effect.status_effect().to_raw() as i32),
                    });
                }
            }
        }

        active_status_effects
            .active
            .retain(|effect| !effect.expired());
    }
}
