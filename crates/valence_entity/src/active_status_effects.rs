use bevy_ecs::prelude::*;

use crate::status_effects::StatusEffect;

/// [`Component`] that stores the [`ActiveStatusEffect`]s of an [`Entity`].
#[derive(Component, Default, Debug)]
pub struct ActiveStatusEffects {
    active_effects: Vec<ActiveStatusEffect>,
    new_effects: Vec<ActiveStatusEffect>,
}

impl ActiveStatusEffects {
    /// Adds a new [`ActiveStatusEffect`] to the [`ActiveStatusEffects`].
    ///
    /// ## Note
    ///
    /// It actually adds the [`ActiveStatusEffect`] to the new effects. The
    /// [`ActiveStatusEffect`] will be added to the active effects in the next
    /// tick (more specifically, during the upcoming [`EventLoopPostUpdate`]).
    /// If the [`ActiveStatusEffect`] is already in the new effects, it will be
    /// replaced.
    ///
    /// [`EventLoopPostUpdate`]: https://valence.rs/rustdoc/valence/struct.EventLoopPostUpdate.html
    pub fn add(&mut self, effect: ActiveStatusEffect) {
        /*
         * Note: We don't remove the effect if it is already active because it
         * would cause inconsistencies. For example, if the effect is already
         * active and we remove it, it's gone under the assumption that it will
         * be replaced by the new effect. However, if the new effect is then
         * removed, the effect is gone forever and the server didn't get the
         * chance to send the remove packet.
         */

        // Remove the effect if it is already in the new effects.
        self.new_effects
            .retain(|new_effect| new_effect.status_effect() != effect.status_effect());

        self.new_effects.push(effect);
    }

    /// Removes an [`ActiveStatusEffect`] from the [`ActiveStatusEffects`].
    ///
    /// ## Note
    ///
    /// If the effect is already active, it actually sets the duration of the
    /// [`ActiveStatusEffect`] to 0. The [`ActiveStatusEffect`] will be
    /// properly removed in the next tick (more specifically, during the
    /// upcoming [`EventLoopPostUpdate`]). Otherwise, it removes it from the
    /// `new_effects`.
    ///
    /// [`EventLoopPostUpdate`]: https://valence.rs/rustdoc/valence/struct.EventLoopPostUpdate.html
    pub fn remove(&mut self, effect: StatusEffect) {
        // It just sets the duration to 0, so it will be properly removed in the next
        // tick.
        if let Some(active_effect) = self
            .active_effects
            .iter_mut()
            .find(|active_effect| active_effect.status_effect() == effect)
        {
            active_effect.duration = Some(0);
        }

        // Remove the effect if it is already in the new effects.
        self.new_effects
            .retain(|new_effect| new_effect.status_effect() != effect);
    }

    /// Returns the [`ActiveStatusEffect`]s of the [`ActiveStatusEffects`].
    ///
    /// ## Note
    ///
    /// Returns an iterator over the active effects and the new effects.
    pub fn active_effects(&self) -> impl Iterator<Item = &ActiveStatusEffect> {
        self.active_effects.iter().chain(self.new_effects.iter())
    }

    /// Returns true if the [`ActiveStatusEffects`] has no active or new
    /// effects.
    pub fn is_empty(&self) -> bool {
        self.active_effects.is_empty() && self.new_effects.is_empty()
    }

    /// Returns the [`ActiveStatusEffect`]s of the [`ActiveStatusEffects`]
    /// mutably.
    ///
    /// # Warning
    ///
    /// This method should only be used by the server. Be careful when modifying
    /// the [`ActiveStatusEffect`]s as it may cause inconsistencies.
    ///
    /// If you want to add, remove or modify [`ActiveStatusEffect`]s, use the
    /// [`add`](ActiveStatusEffects::add) and
    /// [`remove`](ActiveStatusEffects::remove) methods instead of directly
    /// modifying the [`ActiveStatusEffect`]s.
    pub fn active_effects_mut(&mut self) -> &mut Vec<ActiveStatusEffect> {
        &mut self.active_effects
    }

    /// Removes all the [`ActiveStatusEffect`]s from the active effects that are
    /// in the new effects.
    fn remove_new_from_active(&mut self) {
        self.active_effects
            .retain(|active_effect| !self.new_effects.contains(active_effect));
    }

    /// For internal use only. Moves the new effects to the active effects
    /// and returns an iterator over the new effects.
    pub fn move_new_to_active(&mut self) -> impl Iterator<Item = &ActiveStatusEffect> {
        self.remove_new_from_active();

        let old_len = self.active_effects.len();

        self.active_effects.append(&mut self.new_effects);

        self.active_effects[old_len..].iter()
    }

    pub fn remove_expired(&mut self) {
        self.active_effects.retain(|effect| !effect.expired());
    }
}

/// Represents an active status effect.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ActiveStatusEffect {
    effect: StatusEffect,
    /// # Default Value
    /// 0
    amplifier: u8,
    /// # Default Value
    /// 600 ticks (30 seconds)
    duration: Option<i32>,
    /// # Default Value
    /// false
    ambient: bool,
    /// # Default Value
    /// true
    show_particles: bool,
    /// # Default Value
    /// true
    show_icon: bool,
}

impl ActiveStatusEffect {
    /// Creates a new [`ActiveStatusEffect`].
    pub fn from_effect(effect: StatusEffect) -> Self {
        Self {
            effect,
            amplifier: 0,
            duration: Some(600),
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
        self.duration = Some(duration);
        self
    }

    /// Sets the duration of the [`ActiveStatusEffect`] in seconds.
    pub fn with_duration_seconds(mut self, duration: f32) -> Self {
        self.duration = Some((duration * 20.0).round() as i32);
        self
    }

    /// Sets the duration of the [`ActiveStatusEffect`] to infinite.
    pub fn with_infinite(mut self) -> Self {
        self.duration = None;
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
        if let Some(duration) = self.duration.as_mut() {
            *duration -= 1;

            if *duration <= 0 {
                *duration = 0;
            }
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
    /// Returns `None` if the [`ActiveStatusEffect`] is infinite.
    pub fn duration(&self) -> Option<i32> {
        self.duration
    }

    /// Returns true if the [`ActiveStatusEffect`] is ambient.
    pub fn ambient(&self) -> bool {
        self.ambient
    }

    /// Returns true if the [`ActiveStatusEffect`] shows particles.
    pub fn show_particles(&self) -> bool {
        self.show_particles
    }

    /// Returns true if the [`ActiveStatusEffect`] shows an icon.
    pub fn show_icon(&self) -> bool {
        self.show_icon
    }

    /// Returns true if the [`ActiveStatusEffect`] has expired or if it is
    /// instant.
    pub fn expired(&self) -> bool {
        self.status_effect().instant() || self.duration().map_or(false, |duration| duration == 0)
    }
}
