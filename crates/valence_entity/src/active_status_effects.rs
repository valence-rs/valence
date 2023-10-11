use bevy_ecs::prelude::*;
use indexmap::IndexMap;

use crate::status_effects::StatusEffect;

/// Represents a change in the [`ActiveStatusEffects`] of an [`Entity`].
#[derive(Debug)]
enum StatusEffectChange {
    Apply(ActiveStatusEffect),
    Replace(ActiveStatusEffect),
    Remove(StatusEffect),
    RemoveAll,
    /// **For internal use only.**
    Expire(StatusEffect),
}

/// The result of a duration calculation for a status effect.
pub enum DurationResult {
    /// There are no effects of the given type.
    NoEffects,
    /// The effect has an infinite duration.
    Infinite,
    /// The effect has a finite duration, represented as an integer number of
    /// turns.
    Finite(i32),
}

/// [`Component`] that stores the [`ActiveStatusEffect`]s of an [`Entity`].
#[derive(Component, Default, Debug)]
pub struct ActiveStatusEffects {
    /// vec is always sorted in descending order of amplifier and ascending
    /// order of duration.
    current_effects: IndexMap<StatusEffect, Vec<ActiveStatusEffect>>,
    changes: Vec<StatusEffectChange>,
}

/* Public API I imagine:
 * - apply(status_effect: ActiveStatusEffect) // Applies a new effect. Does
 *   it in the same way that the Vanilla server does it. i.e., if the effect
 *   is already active:
 *   1. if the new effect is the same as the old one and its duration is
 *      longer, it replaces the old effect. Otherwise, it does nothing.
 *   2. if the new effect is stronger than the old one:
 *     a. if the new effect's duration is longer, it replaces the old effect.
 *     b. if the new effect's duration is shorter, it overrides the old
 *        effect until the new effect's duration is over.
 *   3. if the new effect is weaker than the old one and if the new effect's
 *      duration is longer, it will be overridden by the old effect until the
 *      old effect's duration is over.
 * - replace(status_effect: ActiveStatusEffect) // Replaces an existing
 *   effect.
 * - remove(status_effect: StatusEffect) // Removes an effect.
 * - remove_all() // Removes all the effects.
 * - get_current_effect(status_effect: StatusEffect) ->
 *   Option<ActiveStatusEffect> // Returns the current active effect. If
 *   there are multiple effects, it // returns the strongest one.
 * - get_all_effect(status_effect: StatusEffect) -> Vec<ActiveStatusEffect>
 *   // Returns all the active effects.
 * - get_current_effects() -> Vec<ActiveStatusEffect> // Returns all the
 *   active effects. If there are multiple effects of the // same type, it
 *   returns the strongest one.
 * - get_all_effects() -> IndexMap<StatusEffect, Vec<ActiveStatusEffect>> //
 *   Returns all the active effects. (is IndexMap really necessary?)
 */
impl ActiveStatusEffects {
    // public API goes here

    /// Applies a new [`ActiveStatusEffect`].
    ///
    /// If the [`ActiveStatusEffect`] is already active:
    /// 1. if the new effect is the same as the old one and its duration is
    ///    longer, it replaces the old effect. Otherwise, it does nothing.
    /// 2. if the new effect is stronger than the old one:
    ///   a. if the new effect's duration is longer, it replaces the old effect.
    ///   b. if the new effect's duration is shorter, it overrides the old
    /// 3. if the new effect is weaker than the old one and if the new effect's
    ///    duration is longer, it will be overridden by the old effect until the
    ///    old effect's duration is over.
    pub fn apply(&mut self, effect: ActiveStatusEffect) {
        self.changes.push(StatusEffectChange::Apply(effect));
    }

    /// Replace an existing [`ActiveStatusEffect`].
    pub fn replace(&mut self, effect: ActiveStatusEffect) {
        self.changes.push(StatusEffectChange::Replace(effect));
    }

    /// Removes an [`ActiveStatusEffect`].
    pub fn remove(&mut self, effect: StatusEffect) {
        self.changes.push(StatusEffectChange::Remove(effect));
    }

    /// Removes all [`ActiveStatusEffect`]s.
    pub fn remove_all(&mut self) {
        self.changes.push(StatusEffectChange::RemoveAll);
    }

    /// Returns true if there are no effects of the given type.
    pub fn no_effect(&self, effect: StatusEffect) -> bool {
        self.current_effects
            .get(&effect)
            .map_or(true, |effects| effects.is_empty())
    }

    /// Returns true if there are no effects.
    pub fn no_effects(&self) -> bool {
        self.current_effects.is_empty()
    }

    /// Returns the maximum duration of the given effect.
    pub fn max_duration(&self, effect: StatusEffect) -> DurationResult {
        let effects = self.current_effects.get(&effect);

        match effects {
            None => DurationResult::NoEffects,
            Some(effects) => {
                if let Some(effect) = effects.last() {
                    match effect.duration() {
                        None => DurationResult::Infinite,
                        Some(duration) => DurationResult::Finite(duration),
                    }
                } else {
                    DurationResult::NoEffects
                }
            }
        }
    }

    /// Gets the current effect of the given type.
    pub fn get_current_effect(&self, effect: StatusEffect) -> Option<&ActiveStatusEffect> {
        self.current_effects
            .get(&effect)
            .and_then(|effects| effects.first())
    }

    /// Gets all the effects of the given type.
    pub fn get_all_effect(&self, effect: StatusEffect) -> Option<&Vec<ActiveStatusEffect>> {
        self.current_effects.get(&effect)
    }

    /// Gets all the current effects.
    pub fn get_current_effects(&self) -> Vec<&ActiveStatusEffect> {
        self.current_effects
            .values()
            .filter_map(|effects| effects.first())
            .collect()
    }

    /// Gets all the effects.
    pub fn get_all_effects(&self) -> &IndexMap<StatusEffect, Vec<ActiveStatusEffect>> {
        &self.current_effects
    }
}

impl ActiveStatusEffects {
    // internal API goes here

    /// Applies an effect.
    ///
    /// The vec must always be sorted in descending order of amplifier and
    /// ascending order of duration.
    ///
    /// Returns true if the effect was applied.
    fn apply_effect(&mut self, effect: ActiveStatusEffect) -> bool {
        let effects = self
            .current_effects
            .entry(effect.status_effect())
            .or_default();

        if let Some(index) = effects
            .iter()
            .position(|e| e.amplifier() < effect.amplifier())
        {
            // Found an effect with a lower amplifier.

            if effects[index].duration() < effect.duration() {
                // if its duration is shorter, override it.
                effects[index] = effect;
                true
            } else {
                // if its duration is longer, insert it before the effect.
                effects.insert(index, effect);
                true
            }
        } else {
            // Didn't find an effect with a lower amplifier.
            // This means that the effect has or is tied for lowest amplifier
            // or that there are no existing effects.

            // Get the last effect.
            if let Some(last_effect) = effects.last() {
                let last_index = effects.len() - 1;
                if last_effect.duration() < effect.duration() {
                    // if its duration is longer...
                    if last_effect.amplifier() == effect.amplifier() {
                        // and if it has the same amplifier, override it.
                        effects[last_index] = effect;
                        true
                    } else {
                        // and if it has a different amplifier, insert it after
                        effects.push(effect);
                        true
                    }
                } else {
                    // if its duration is shorter, do nothing. It'll vanish
                    // before it does anything anyway.
                    false
                }
            } else {
                // There are no existing effects.
                effects.push(effect);
                true
            }
        }
    }

    /// Replaces an effect.
    fn replace_effect(&mut self, effect: ActiveStatusEffect) {
        self.current_effects
            .insert(effect.status_effect(), vec![effect]);
    }

    /// Removes an effect.
    fn remove_effect(&mut self, effect: StatusEffect) {
        self.current_effects.remove(&effect);
    }

    /// Removes all effects.
    fn remove_all_effects(&mut self) {
        self.current_effects.clear();
    }

    /// Removes the strongest effect of the given type, i.e., the first effect
    fn remove_strongest_effect(&mut self, effect: StatusEffect) {
        if let Some(effects) = self.current_effects.get_mut(&effect) {
            effects.remove(0);
        }
    }

    /// **For internal use only.**
    ///
    /// Decrements the duration of all effects by a tick.
    pub fn decrement_duration(&mut self) {
        for effects in self.current_effects.values_mut() {
            for effect in effects.iter_mut() {
                effect.decrement_duration();

                if effect.expired() {
                    self.changes
                        .push(StatusEffectChange::Expire(effect.status_effect()));
                }
            }
        }
    }

    /// **For internal use only.**
    ///
    /// Applies all the changes.
    ///
    /// Returns a [`Vec`] of [`StatusEffect`]s that were updated or removed.
    pub fn apply_changes(&mut self) -> Vec<StatusEffect> {
        let mut updated_effects = Vec::new();

        for change in std::mem::take(&mut self.changes) {
            match change {
                StatusEffectChange::Apply(effect) => {
                    let value = effect.status_effect();
                    if self.apply_effect(effect) {
                        updated_effects.push(value);
                    }
                }
                StatusEffectChange::Replace(effect) => {
                    updated_effects.push(effect.status_effect());
                    self.replace_effect(effect);
                }
                StatusEffectChange::Remove(effect) => {
                    self.remove_effect(effect);
                    updated_effects.push(effect);
                }
                StatusEffectChange::RemoveAll => {
                    self.remove_all_effects();
                    updated_effects.extend(self.current_effects.keys());
                }
                StatusEffectChange::Expire(effect) => {
                    self.remove_strongest_effect(effect);
                    updated_effects.push(effect);
                }
            }
        }

        updated_effects
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
