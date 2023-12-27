use std::collections::HashMap;

use bevy_ecs::prelude::*;
use indexmap::IndexMap;
use uuid::Uuid;
use valence_protocol::packets::play::entity_attributes_s2c::*;
use valence_protocol::Ident;

use crate::EntityAttribute;

/// An instance of an Entity Attribute.
#[derive(Component, Clone, PartialEq, Debug)]
pub struct EntityAttributeInstance {
    /// The attribute.
    attribute: EntityAttribute,
    /// The base value of the attribute.
    base_value: f64,
    /// The add modifiers of the attribute.
    add_modifiers: IndexMap<Uuid, f64>,
    /// The multiply base modifiers of the attribute.
    multiply_base_modifiers: IndexMap<Uuid, f64>,
    /// The multiply total modifiers of the attribute.
    multiply_total_modifiers: IndexMap<Uuid, f64>,
}

impl EntityAttributeInstance {
    /// Creates a new instance of an Entity Attribute.
    pub fn new(attribute: EntityAttribute) -> Self {
        Self {
            attribute,
            base_value: attribute.default_value(),
            add_modifiers: IndexMap::new(),
            multiply_base_modifiers: IndexMap::new(),
            multiply_total_modifiers: IndexMap::new(),
        }
    }

    /// Creates a new instance of an Entity Attribute with a value.
    pub fn new_with_value(attribute: EntityAttribute, base_value: f64) -> Self {
        Self {
            attribute,
            base_value,
            add_modifiers: IndexMap::new(),
            multiply_base_modifiers: IndexMap::new(),
            multiply_total_modifiers: IndexMap::new(),
        }
    }

    /// Gets the attribute.
    pub fn attribute(&self) -> EntityAttribute {
        self.attribute
    }

    /// Gets the base value of the attribute.
    pub fn base_value(&self) -> f64 {
        self.base_value
    }

    /// Gets the computed value of the attribute.
    pub fn compute_value(&self) -> f64 {
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

        value.clamp(self.attribute.min_value(), self.attribute.max_value())
    }

    /// Sets an add modifier.
    ///
    /// If the modifier already exists, it will be overwritten.
    ///
    /// Returns a mutable reference to self.
    pub fn with_add_modifier(&mut self, uuid: Uuid, modifier: f64) -> &mut Self {
        self.add_modifiers.insert(uuid, modifier);
        self
    }

    /// Sets a multiply base modifier.
    ///
    /// If the modifier already exists, it will be overwritten.
    ///
    /// Returns a mutable reference to self.
    pub fn with_multiply_base_modifier(&mut self, uuid: Uuid, modifier: f64) -> &mut Self {
        self.multiply_base_modifiers.insert(uuid, modifier);
        self
    }

    /// Sets a multiply total modifier.
    ///
    /// If the modifier already exists, it will be overwritten.
    ///
    /// Returns a mutable reference to self.
    pub fn with_multiply_total_modifier(&mut self, uuid: Uuid, modifier: f64) -> &mut Self {
        self.multiply_total_modifiers.insert(uuid, modifier);
        self
    }

    /// Removes a modifier.
    pub fn remove_modifier(&mut self, uuid: Uuid) {
        self.add_modifiers.remove(&uuid);
        self.multiply_base_modifiers.remove(&uuid);
        self.multiply_total_modifiers.remove(&uuid);
    }

    /// Clears all modifiers.
    pub fn clear_modifiers(&mut self) {
        self.add_modifiers.clear();
        self.multiply_base_modifiers.clear();
        self.multiply_total_modifiers.clear();
    }

    /// Checks if a modifier exists.
    pub fn has_modifier(&self, uuid: Uuid) -> bool {
        self.add_modifiers.contains_key(&uuid)
            || self.multiply_base_modifiers.contains_key(&uuid)
            || self.multiply_total_modifiers.contains_key(&uuid)
    }

    /// Converts to a `TrackedEntityProperty` for use in the
    /// `EntityAttributesS2c` packet.
    pub(crate) fn to_property(&self) -> TrackedEntityProperty {
        TrackedEntityProperty {
            key: self.attribute.name().into(),
            value: self.base_value(),
            modifiers: self
                .add_modifiers
                .iter()
                .map(|(&uuid, &amount)| TrackedAttributeModifier {
                    uuid,
                    amount,
                    operation: 0,
                })
                .chain(self.multiply_base_modifiers.iter().map(|(&uuid, &amount)| {
                    TrackedAttributeModifier {
                        uuid,
                        amount,
                        operation: 1,
                    }
                }))
                .chain(
                    self.multiply_total_modifiers
                        .iter()
                        .map(|(&uuid, &amount)| TrackedAttributeModifier {
                            uuid,
                            amount,
                            operation: 2,
                        }),
                )
                .collect(),
        }
    }
}

/// The attributes of a Living Entity.
#[derive(Component, Clone, PartialEq, Debug, Default)]
pub struct EntityAttributes {
    attributes: HashMap<EntityAttribute, EntityAttributeInstance>,
    recently_changed: Vec<EntityAttribute>,
}

impl EntityAttributes {
    /// Gets and clears the recently changed attributes.
    pub(crate) fn take_recently_changed(&mut self) -> Vec<EntityAttribute> {
        std::mem::take(&mut self.recently_changed)
    }

    /// Marks an attribute as recently changed.
    pub(crate) fn mark_recently_changed(&mut self, attribute: EntityAttribute) {
        if attribute.tracked() && !self.recently_changed.contains(&attribute) {
            self.recently_changed.push(attribute);
        }
    }
}

impl EntityAttributes {
    /// Creates a new instance of EntityAttributes.
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
            recently_changed: Vec::new(),
        }
    }

    /// Gets the instance of an attribute.
    pub fn get(&self, attribute: EntityAttribute) -> Option<&EntityAttributeInstance> {
        self.attributes.get(&attribute)
    }

    /// Gets the base value of an attribute.
    ///
    /// Returns [`None`] if the attribute does not exist.
    pub fn get_base_value(&self, attribute: EntityAttribute) -> Option<f64> {
        self.get(attribute).map(|instance| instance.base_value())
    }

    /// Gets the computed value of an attribute.
    ///
    /// Returns [`None`] if the attribute does not exist.
    pub fn get_compute_value(&self, attribute: EntityAttribute) -> Option<f64> {
        self.get(attribute).map(|instance| instance.compute_value())
    }

    /// Checks if an attribute exists.
    pub fn has_attribute(&self, attribute: EntityAttribute) -> bool {
        self.attributes.contains_key(&attribute)
    }

    /// Creates an attribute if it does not exist.
    pub fn create_attribute(&mut self, attribute: EntityAttribute) {
        self.mark_recently_changed(attribute);
        self.attributes
            .entry(attribute)
            .or_insert_with(|| EntityAttributeInstance::new(attribute));
    }

    /// Creates an attribute if it does not exist and sets its base value.
    ///
    /// Returns self.
    ///
    /// ## Note
    ///
    /// Only to be used in builder-like patterns.
    pub(crate) fn with_attribute_and_value(
        mut self,
        attribute: EntityAttribute,
        base_value: f64,
    ) -> Self {
        self.attributes
            .entry(attribute)
            .or_insert_with(|| EntityAttributeInstance::new_with_value(attribute, base_value))
            .base_value = base_value;
        self
    }

    /// Sets the base value of an attribute.
    pub fn set_base_value(&mut self, attribute: EntityAttribute, value: f64) {
        self.mark_recently_changed(attribute);
        self.attributes
            .entry(attribute)
            .or_insert_with(|| EntityAttributeInstance::new(attribute))
            .base_value = value;
    }

    /// Sets an add modifier of an attribute.
    pub fn set_add_modifier(&mut self, attribute: EntityAttribute, uuid: Uuid, modifier: f64) {
        self.mark_recently_changed(attribute);
        self.attributes
            .entry(attribute)
            .or_insert_with(|| EntityAttributeInstance::new(attribute))
            .with_add_modifier(uuid, modifier);
    }

    /// Sets a multiply base modifier of an attribute.
    pub fn set_multiply_base_modifier(
        &mut self,
        attribute: EntityAttribute,
        uuid: Uuid,
        modifier: f64,
    ) {
        self.mark_recently_changed(attribute);
        self.attributes
            .entry(attribute)
            .or_insert_with(|| EntityAttributeInstance::new(attribute))
            .with_multiply_base_modifier(uuid, modifier);
    }

    /// Sets a multiply total modifier of an attribute.
    pub fn set_multiply_total_modifier(
        &mut self,
        attribute: EntityAttribute,
        uuid: Uuid,
        modifier: f64,
    ) {
        self.mark_recently_changed(attribute);
        self.attributes
            .entry(attribute)
            .or_insert_with(|| EntityAttributeInstance::new(attribute))
            .with_multiply_total_modifier(uuid, modifier);
    }

    /// Removes a modifier of an attribute.
    pub fn remove_modifier(&mut self, attribute: EntityAttribute, uuid: Uuid) {
        self.mark_recently_changed(attribute);
        if let Some(instance) = self.attributes.get_mut(&attribute) {
            instance.remove_modifier(uuid);
        }
    }

    /// Clears all modifiers of an attribute.
    pub fn clear_modifiers(&mut self, attribute: EntityAttribute) {
        self.mark_recently_changed(attribute);
        if let Some(instance) = self.attributes.get_mut(&attribute) {
            instance.clear_modifiers();
        }
    }

    /// Checks if a modifier exists on an attribute.
    pub fn has_modifier(&self, attribute: EntityAttribute, uuid: Uuid) -> bool {
        self.attributes
            .get(&attribute)
            .map(|instance| instance.has_modifier(uuid))
            .unwrap_or(false)
    }

    /// **For internal use only.**
    ///
    /// Converts to a [`Vec`] of [`AttributeProperty`]s.
    pub fn to_properties(&self) -> Vec<AttributeProperty> {
        self.attributes
            .iter()
            .filter(|(_, instance)| instance.attribute().tracked())
            .map(|(_, instance)| instance.to_property().to_property())
            .collect()
    }
}

/// Tracks the attributes of a Living Entity.
#[derive(Component, Clone, Debug, Default)]
pub struct TrackedEntityAttributes {
    /// The attributes that have been modified.
    modified: IndexMap<EntityAttribute, TrackedEntityProperty>,
}

#[derive(Clone, Debug)]
pub(crate) struct TrackedEntityProperty {
    key: String,
    value: f64,
    modifiers: Vec<TrackedAttributeModifier>,
}

#[derive(Clone, Debug)]
pub(crate) struct TrackedAttributeModifier {
    uuid: Uuid,
    amount: f64,
    operation: u8,
}

impl TrackedEntityProperty {
    /// Converts to an [`AttributeProperty`]s.
    fn to_property(&self) -> AttributeProperty<'static> {
        AttributeProperty {
            key: Ident::new(self.key.clone()).unwrap(),
            value: self.value,
            modifiers: self
                .modifiers
                .iter()
                .map(|modifier| AttributeModifier {
                    uuid: modifier.uuid,
                    amount: modifier.amount,
                    operation: modifier.operation,
                })
                .collect(),
        }
    }
}

impl TrackedEntityAttributes {
    /// Creates a new instance of TrackedEntityAttributes.
    pub fn new() -> Self {
        Self {
            modified: IndexMap::new(),
        }
    }

    /// Marks an attribute as modified.
    pub fn mark_modified(&mut self, attributes: &EntityAttributes, attribute: EntityAttribute) {
        if let Some(instance) = attributes.get(attribute) {
            self.modified.insert(attribute, instance.to_property());
        }
    }

    /// Returns the properties turned into a [`Vec`] of [`AttributeProperty`]s.
    pub fn get_properties(&self) -> Vec<AttributeProperty<'static>> {
        self.modified
            .iter()
            .map(|(_, property)| property.to_property())
            .collect()
    }

    /// Clears the modified attributes.
    pub fn clear(&mut self) {
        self.modified.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_value() {
        let add_uuid = Uuid::new_v4();
        let mut attributes = EntityAttributes::new();
        attributes.set_base_value(EntityAttribute::GenericMaxHealth, 20.0);
        attributes.set_add_modifier(EntityAttribute::GenericMaxHealth, add_uuid, 10.0);
        attributes.set_multiply_base_modifier(
            EntityAttribute::GenericMaxHealth,
            Uuid::new_v4(),
            0.2,
        );
        attributes.set_multiply_base_modifier(
            EntityAttribute::GenericMaxHealth,
            Uuid::new_v4(),
            0.2,
        );
        attributes.set_multiply_total_modifier(
            EntityAttribute::GenericMaxHealth,
            Uuid::new_v4(),
            0.5,
        );

        assert_eq!(
            attributes.get_compute_value(EntityAttribute::GenericMaxHealth),
            Some(63.0) // ((20 + 10) * (1 + 0.2 + 0.2)) * (1 + 0.5)
        );

        attributes.remove_modifier(EntityAttribute::GenericMaxHealth, add_uuid);

        assert_eq!(
            attributes.get_compute_value(EntityAttribute::GenericMaxHealth),
            Some(42.0) // ((20) * (1 + 0.2 + 0.2)) * (1 + 0.5)
        );
    }
}
