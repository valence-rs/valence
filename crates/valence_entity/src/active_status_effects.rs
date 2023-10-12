use std::collections::HashSet;

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
                    match effect.remaining_duration() {
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

        let duration = effect.remaining_duration();
        let amplifier = effect.amplifier();

        if let Some(index) = effects.iter().position(|e| e.amplifier() <= amplifier) {
            // Found an effect with the same or a lower amplifier.

            let active_status_effect = &effects[index];

            if active_status_effect.remaining_duration() < duration {
                // if its duration is shorter, override it.
                effects[index] = effect;

                // Remove effects after the current one that have a lower
                // duration.
                let mut remaining_effects = effects.split_off(index + 1);
                remaining_effects.retain(|e| e.remaining_duration() >= duration);
                effects.append(&mut remaining_effects);
                true
            } else if active_status_effect.remaining_duration() > duration
                && active_status_effect.amplifier() < amplifier
            {
                // if its duration is longer and its amplifier is lower, insert
                // the new effect before it.
                effects.insert(index, effect);
                true
            } else {
                // if its duration is longer and its amplifier is higher, do
                // nothing.
                false
            }
        } else {
            // Found no effect with an equal or lower amplifier.
            // This means all effects have a higher amplifier or the vec is
            // empty.

            if let Some(last) = effects.last() {
                println!("BBBB");
                // There is at least one effect with a higher amplifier.
                if last.remaining_duration() < effect.remaining_duration() {
                    // if its duration is shorter, we can insert it at the end.
                    effects.push(effect);
                    true
                } else {
                    // if its duration is longer, do nothing.
                    false
                }
            } else {
                // The vec is empty.
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
    /// Increments the active tick of all effects by a tick.
    pub fn increment_active_ticks(&mut self) {
        for effects in self.current_effects.values_mut() {
            for effect in effects.iter_mut() {
                effect.increment_active_ticks();

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
    /// Returns a [`HashSet`] of [`StatusEffect`]s that were updated or removed.
    pub fn apply_changes(&mut self) -> HashSet<StatusEffect> {
        let mut updated_effects = HashSet::new();

        for change in std::mem::take(&mut self.changes) {
            match change {
                StatusEffectChange::Apply(effect) => {
                    let value = effect.status_effect();
                    if self.apply_effect(effect) {
                        updated_effects.insert(value);
                    }
                }
                StatusEffectChange::Replace(effect) => {
                    updated_effects.insert(effect.status_effect());
                    self.replace_effect(effect);
                }
                StatusEffectChange::Remove(effect) => {
                    self.remove_effect(effect);
                    updated_effects.insert(effect);
                }
                StatusEffectChange::RemoveAll => {
                    self.remove_all_effects();
                    updated_effects.extend(self.current_effects.keys());
                }
                StatusEffectChange::Expire(effect) => {
                    self.remove_strongest_effect(effect);
                    updated_effects.insert(effect);
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
    initial_duration: Option<i32>,
    /// The amount of ticks the effect has been active.
    ///
    /// # Default Value
    /// 0
    active_ticks: i32,
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
            initial_duration: Some(600),
            active_ticks: 0,
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
        self.initial_duration = Some(duration);
        self
    }

    /// Sets the duration of the [`ActiveStatusEffect`] in seconds.
    pub fn with_duration_seconds(mut self, duration: f32) -> Self {
        self.initial_duration = Some((duration * 20.0) as i32);
        self
    }

    /// Sets the duration of the [`ActiveStatusEffect`] to infinite.
    pub fn with_infinite(mut self) -> Self {
        self.initial_duration = None;
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

    /// Increments the active ticks of the [`ActiveStatusEffect`] by one.
    pub fn increment_active_ticks(&mut self) {
        self.active_ticks += 1;
    }

    /// Returns the [`StatusEffect`] of the [`ActiveStatusEffect`].
    pub fn status_effect(&self) -> StatusEffect {
        self.effect
    }

    /// Returns the amplifier of the [`ActiveStatusEffect`].
    pub fn amplifier(&self) -> u8 {
        self.amplifier
    }

    /// Returns the initial duration of the [`ActiveStatusEffect`] in ticks.
    /// Returns `None` if the [`ActiveStatusEffect`] is infinite.
    pub fn initial_duration(&self) -> Option<i32> {
        self.initial_duration
    }

    /// Returns the remaining duration of the [`ActiveStatusEffect`] in ticks.
    /// Returns `None` if the [`ActiveStatusEffect`] is infinite.
    pub fn remaining_duration(&self) -> Option<i32> {
        self.initial_duration
            .map(|duration| duration - self.active_ticks)
    }

    /// Returns the active ticks of the [`ActiveStatusEffect`].
    pub fn active_ticks(&self) -> i32 {
        self.active_ticks
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
        self.status_effect().instant()
            || self
                .remaining_duration()
                .map_or(false, |duration| duration <= 0)
    }

    /// Returns true if the [`ActiveStatusEffect`] can be applied to an
    /// [`Entity`]. Yoinked from the Vanilla server.
    pub fn can_apply_update_effect(&self) -> bool {
        match self.status_effect() {
            StatusEffect::Regeneration => {
                let i = 50 >> self.amplifier;
                if i > 0 {
                    return self.active_ticks % i == 0;
                }
                true
            }
            StatusEffect::Poison => {
                let i = 25 >> self.amplifier;
                if i > 0 {
                    return self.active_ticks % i == 0;
                }
                true
            }
            StatusEffect::Wither => {
                let i = 40 >> self.amplifier;
                if i > 0 {
                    return self.active_ticks % i == 0;
                }
                true
            }
            StatusEffect::Hunger => true,
            _ => false,
        }
    }

    // TODO: Implement health changes and hunger changes.
}
