use std::collections::BTreeMap;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tracing::error;
use valence_ident::Ident;
use valence_nbt::{compound, Compound, List, Value};

use crate::UpdateRegistrySet;

pub(super) fn build(app: &mut App) {
    app.init_resource::<RegistryCodec>()
        .add_systems(PostUpdate, cache_registry_codec.in_set(UpdateRegistrySet));
}

/// Contains the registry codec sent to all players while joining. This contains
/// information for biomes and dimensions among other things.
///
/// Generally, end users should not manipulate the registry codec directly. Use
/// one of the other registry resources instead.
#[derive(Resource, Debug)]
pub struct RegistryCodec {
    pub registries: BTreeMap<Ident<String>, Vec<RegistryValue>>,
    // TODO: store this in binary form?
    cached_codec: Compound,
}

#[derive(Clone, Debug)]
pub struct RegistryValue {
    pub name: Ident<String>,
    pub element: Compound,
}

impl RegistryCodec {
    pub fn cached_codec(&self) -> &Compound {
        &self.cached_codec
    }

    pub fn registry(&self, registry_key: Ident<&str>) -> &Vec<RegistryValue> {
        self.registries
            .get(registry_key.as_str())
            .unwrap_or_else(|| panic!("missing registry for {registry_key}"))
    }

    pub fn registry_mut(&mut self, registry_key: Ident<&str>) -> &mut Vec<RegistryValue> {
        self.registries
            .get_mut(registry_key.as_str())
            .unwrap_or_else(|| panic!("missing registry for {registry_key}"))
    }
}

impl Default for RegistryCodec {
    fn default() -> Self {
        let codec = include_bytes!("../extracted/registry_codec.dat");
        let compound = valence_nbt::from_binary(&mut codec.as_slice())
            .expect("failed to decode vanilla registry codec")
            .0;

        let mut registries = BTreeMap::new();

        for (k, v) in compound {
            let reg_name: Ident<String> = Ident::new(k).expect("invalid registry name").into();
            let mut reg_values = vec![];

            let Value::Compound(mut outer) = v else {
                error!("registry {reg_name} is not a compound");
                continue;
            };

            let values = match outer.remove("value") {
                Some(Value::List(List::Compound(values))) => values,
                Some(Value::List(List::End)) => continue,
                _ => {
                    error!("missing \"value\" compound in {reg_name}");
                    continue;
                }
            };

            for mut value in values {
                let Some(Value::String(name)) = value.remove("name") else {
                    error!("missing \"name\" string in value for {reg_name}");
                    continue;
                };

                let name = match Ident::new(name) {
                    Ok(n) => n.into(),
                    Err(e) => {
                        error!("invalid registry value name \"{}\"", e.0);
                        continue;
                    }
                };

                let Some(Value::Compound(element)) = value.remove("element") else {
                    error!("missing \"element\" compound in value for {reg_name}");
                    continue;
                };

                reg_values.push(RegistryValue { name, element });
            }

            registries.insert(reg_name, reg_values);
        }

        Self {
            registries,
            // Cache will be created later.
            cached_codec: Compound::new(),
        }
    }
}

fn cache_registry_codec(codec: ResMut<RegistryCodec>) {
    if codec.is_changed() {
        let codec = codec.into_inner();

        codec.cached_codec.clear();

        for (reg_name, reg) in &codec.registries {
            let mut value = vec![];

            for (id, v) in reg.iter().enumerate() {
                value.push(compound! {
                    "id" => id as i32,
                    "name" => v.name.as_str(),
                    "element" => v.element.clone(),
                });
            }

            let registry = compound! {
                "type" => reg_name.as_str(),
                "value" => List::Compound(value),
            };

            codec.cached_codec.insert(reg_name.as_str(), registry);
        }
    }
}
