use bevy_ecs::prelude::*;
use valence_protocol::StatusEffect;

/// [`Component`] that stores the [`ActiveStatusEffect`]s of an [`Entity`].
#[derive(Component, Default)]
pub struct ActiveStatusEffects {
    active: Vec<StatusEffect>,
    new: Vec<StatusEffect>,
}

impl ActiveStatusEffects {
    pub fn add(&mut self, effect: StatusEffect) {
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
        self.duration -= 1;
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
        self.duration <= 0
    }
}
