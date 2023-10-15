use std::collections::HashMap;

use bevy_ecs::prelude::*;
use indexmap::IndexMap;

use crate::EntityAttribute;

/// An instance of an Entity Attribute.
#[derive(Component, Clone, PartialEq, Debug)]
pub struct EntityAttributeInstance {
    /// The base value of the attribute.
    pub base_value: f32,
    /// The add modifiers of the attribute.
    pub add_modifiers: IndexMap<String, f32>,
    /// The multiply base modifiers of the attribute.
    pub multiply_base_modifiers: IndexMap<String, f32>,
    /// The multiply total modifiers of the attribute.
    pub multiply_total_modifiers: IndexMap<String, f32>,
}

impl EntityAttributeInstance {
    /// Creates a new instance of an Entity Attribute.
    pub fn new(base_value: f32) -> Self {
        Self {
            base_value,
            add_modifiers: IndexMap::new(),
            multiply_base_modifiers: IndexMap::new(),
            multiply_total_modifiers: IndexMap::new(),
        }
    }

    /// Gets the value of the attribute.
    pub fn value(&self) -> f32 {
        let mut value = self.base_value;

        // Increment value by modifier
        for (_, modifier) in self.add_modifiers.iter() {
            value += modifier;
        }

        let v = value;

        // Increment value by modifier * v
        for (_, modifier) in self.multiply_base_modifiers.iter() {
            value += v * modifier;
        }

        // Increment value by modifier * value
        for (_, modifier) in self.multiply_total_modifiers.iter() {
            value += value * modifier;
        }

        value
    }

    /// Sets an add modifier.
    pub fn set_add_modifier(&mut self, name: String, modifier: f32) {
        self.add_modifiers.insert(name, modifier);
    }

    /// Sets a multiply base modifier.
    pub fn set_multiply_base_modifier(&mut self, name: String, modifier: f32) {
        self.multiply_base_modifiers.insert(name, modifier);
    }

    /// Sets a multiply total modifier.
    pub fn set_multiply_total_modifier(&mut self, name: String, modifier: f32) {
        self.multiply_total_modifiers.insert(name, modifier);
    }
    
    /// Removes a modifier.
    pub fn remove_modifier(&mut self, name: &str) {
        self.add_modifiers.remove(name);
        self.multiply_base_modifiers.remove(name);
        self.multiply_total_modifiers.remove(name);
    }

    /// Clears all modifiers.
    pub fn clear_modifiers(&mut self) {
        self.add_modifiers.clear();
        self.multiply_base_modifiers.clear();
        self.multiply_total_modifiers.clear();
    }
}

/// The attributes of a Living Entity.
#[derive(Component, Clone, PartialEq, Debug)]
pub struct EntityAttributes(HashMap<EntityAttribute, EntityAttributeInstance>);

impl EntityAttributes {
    /// Creates a new instance of EntityAttributes.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Gets the value of an attribute.
    /// 
    /// Returns [`None`] if the attribute does not exist.
    pub fn get_value(&self, attribute: EntityAttribute) -> Option<f32> {
        self.0.get(&attribute).map(|instance| instance.value())
    }

    /// Sets the base value of an attribute.
    pub fn set_base_value(&mut self, attribute: EntityAttribute, value: f32) {
        self.0.entry(attribute).or_insert_with(|| EntityAttributeInstance::new(value)).base_value = value;
    }

    /// Sets an add modifier of an attribute.
    pub fn set_add_modifier(&mut self, attribute: EntityAttribute, name: String, modifier: f32) {
        self.0.entry(attribute).or_insert_with(|| EntityAttributeInstance::new(0.0)).set_add_modifier(name, modifier);
    }

    /// Sets a multiply base modifier of an attribute.
    pub fn set_multiply_base_modifier(&mut self, attribute: EntityAttribute, name: String, modifier: f32) {
        self.0.entry(attribute).or_insert_with(|| EntityAttributeInstance::new(0.0)).set_multiply_base_modifier(name, modifier);
    }

    /// Sets a multiply total modifier of an attribute.
    pub fn set_multiply_total_modifier(&mut self, attribute: EntityAttribute, name: String, modifier: f32) {
        self.0.entry(attribute).or_insert_with(|| EntityAttributeInstance::new(0.0)).set_multiply_total_modifier(name, modifier);
    }

    /// Removes a modifier of an attribute.
    pub fn remove_modifier(&mut self, attribute: EntityAttribute, name: &str) {
        if let Some(instance) = self.0.get_mut(&attribute) {
            instance.remove_modifier(name);
        }
    }

    
}
