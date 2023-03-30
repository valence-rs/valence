//! Biome configuration and identification.
//!
//! **NOTE:**
//!
//! - Modifying the biome registry after the server has started can
//! break invariants within instances and clients! Make sure there are no
//! instances or clients spawned before mutating.
//! - A biome named "minecraft:plains" must exist. Otherwise, vanilla clients
//!   will be disconnected. A biome named "minecraft:plains" is added by
//!   default.

use std::ops::Index;

use anyhow::{bail, Context};
use bevy_app::{CoreSet, Plugin, StartupSet};
use bevy_ecs::prelude::*;
use tracing::error;
use valence_nbt::{compound, Value};
use valence_protocol::ident;
use valence_protocol::ident::Ident;

use crate::registry_codec::{RegistryCodec, RegistryCodecSet, RegistryValue};

#[derive(Resource)]
pub struct BiomeRegistry {
    id_to_biome: Vec<Entity>,
}

impl BiomeRegistry {
    pub const KEY: Ident<&str> = ident!("minecraft:worldgen/biome");

    pub fn get_by_id(&self, id: BiomeId) -> Option<Entity> {
        self.id_to_biome.get(id.0 as usize).cloned()
    }

    pub fn biomes(&self) -> impl Iterator<Item = (BiomeId, Entity)> + '_ {
        self.id_to_biome
            .iter()
            .enumerate()
            .map(|(id, biome)| (BiomeId(id as _), *biome))
    }
}

impl Index<BiomeId> for BiomeRegistry {
    type Output = Entity;

    fn index(&self, index: BiomeId) -> &Self::Output {
        self.id_to_biome
            .get(index.0 as usize)
            .unwrap_or_else(|| panic!("invalid {index:?}"))
    }
}

/// An index into the biome registry.
#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct BiomeId(pub u16);

#[derive(Component, Clone, Debug)]
pub struct Biome {
    pub name: Ident<String>,
    pub downfall: f32,
    pub fog_color: i32,
    pub sky_color: i32,
    pub water_color: i32,
    pub water_fog_color: i32,
    pub has_precipitation: bool,
    pub temperature: f32,
    // TODO: more stuff.
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            name: ident!("minecraft:plains").into(),
            downfall: 0.4,
            fog_color: 12638463,
            sky_color: 7907327,
            water_color: 4159204,
            water_fog_color: 329011,
            has_precipitation: true,
            temperature: 0.8,
        }
    }
}

pub(crate) struct BiomePlugin;

impl Plugin for BiomePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.insert_resource(BiomeRegistry {
            id_to_biome: vec![],
        })
        .add_systems(
            (update_biome_registry, remove_biomes_from_registry)
                .chain()
                .in_base_set(CoreSet::PostUpdate)
                .before(RegistryCodecSet),
        )
        .add_system(load_default_biomes.in_base_set(StartupSet::PreStartup));
    }
}

fn load_default_biomes(
    mut reg: ResMut<BiomeRegistry>,
    codec: Res<RegistryCodec>,
    mut commands: Commands,
) {
    let mut helper = move || {
        for value in codec.registry(BiomeRegistry::KEY) {
            let downfall = *value
                .element
                .get("downfall")
                .and_then(|v| v.as_float())
                .context("invalid downfall")?;

            let Some(Value::Compound(effects)) = value.element.get("effects") else {
                bail!("missing biome effects")
            };

            let fog_color = *effects
                .get("fog_color")
                .and_then(|v| v.as_int())
                .context("invalid fog color")?;

            let sky_color = *effects
                .get("sky_color")
                .and_then(|v| v.as_int())
                .context("invalid sky color")?;

            let water_color = *effects
                .get("water_color")
                .and_then(|v| v.as_int())
                .context("invalid water color")?;

            let water_fog_color = *effects
                .get("water_fog_color")
                .and_then(|v| v.as_int())
                .context("invalid water fog color")?;

            let has_precipitation = *value
                .element
                .get("has_precipitation")
                .and_then(|v| v.as_byte())
                .context("invalid has_precipitation")?
                != 0;

            let temperature = *value
                .element
                .get("temperature")
                .and_then(|v| v.as_float())
                .context("invalid temperature")?;

            let entity = commands
                .spawn(Biome {
                    name: value.name.clone(),
                    downfall,
                    fog_color,
                    sky_color,
                    water_color,
                    water_fog_color,
                    has_precipitation,
                    temperature,
                })
                .id();

            reg.id_to_biome.push(entity);
        }

        Ok(())
    };

    if let Err(e) = helper() {
        error!("failed to load default biomes from registry codec: {e:#}");
    }
}

/// Add new biomes to or update existing biomes in the registry.
fn update_biome_registry(
    mut reg: ResMut<BiomeRegistry>,
    mut codec: ResMut<RegistryCodec>,
    biomes: Query<(Entity, &Biome), Changed<Biome>>,
) {
    for (entity, biome) in &biomes {
        let biome_registry = codec.registry_mut(BiomeRegistry::KEY);

        let biome_compound = compound! {
            "downfall" => biome.downfall,
            "effects" => compound! {
                "fog_color" => biome.fog_color,
                "sky_color" => biome.sky_color,
                "water_color" => biome.water_color,
                "water_fog_color" => biome.water_fog_color,
            },
            "has_precipitation" => biome.has_precipitation,
            "temperature" => biome.temperature,
        };

        if let Some(value) = biome_registry.iter_mut().find(|v| v.name == biome.name) {
            value.name = biome.name.clone();
            value.element.insert_all(biome_compound);
        } else {
            biome_registry.push(RegistryValue {
                name: biome.name.clone(),
                element: biome_compound,
            });
            reg.id_to_biome.push(entity);
        }

        assert_eq!(
            biome_registry.len(),
            reg.id_to_biome.len(),
            "biome registry and biome lookup table differ in length"
        );
    }
}

/// Remove deleted biomes from the registry.
fn remove_biomes_from_registry(
    mut biomes: RemovedComponents<Biome>,
    mut reg: ResMut<BiomeRegistry>,
    mut codec: ResMut<RegistryCodec>,
) {
    for biome in biomes.iter() {
        if let Some(idx) = reg.id_to_biome.iter().position(|entity| *entity == biome) {
            reg.id_to_biome.remove(idx);
            codec.registry_mut(BiomeRegistry::KEY).remove(idx);
        }
    }
}

/*
impl BiomeRegistry {
    pub const KEY: Ident<&str> = ident_str!("minecraft:worldgen/biome");

    pub fn get_by_name(&self, name: Ident<&str>) -> Option<Entity> {
        self.name_to_biome.get(name.as_str()).cloned()
    }

    pub fn get_by_id(&self, id: BiomeId) -> Option<Entity> {
        self.id_to_biome.get(id.0 as usize).cloned()
    }
}

#[derive(Component, Clone, Debug)]
pub struct Biome {
    pub name: Ident<String>,
    pub downfall: f32,
    pub fog_color: i32,
    pub sky_color: i32,
    pub water_color: i32,
    pub water_fog_color: i32,
    pub has_precipitation: bool,
    pub temperature: f32,
    // TODO: more stuff.
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            name: ident!("minecraft:plains"),
            downfall: 0.4,
            fog_color: 12638463,
            sky_color: 7907327,
            water_color: 4159204,
            water_fog_color: 329011,
            has_precipitation: true,
            temperature: 0.8,
        }
    }
}

/// An index into the biome registry.
#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct BiomeId(pub u16);

pub(crate) struct BiomePlugin;

impl Plugin for BiomePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let mut reg = BiomeRegistry {
            name_to_biome: HashMap::new(),
            id_to_biome: vec![],
        };

        let codec = app.world.resource::<RegistryCodec>();

        app.insert_resource(reg).add_system(
            update_biome_registry
                .in_base_set(CoreSet::PostUpdate)
                .before(RegistryCodecSet),
        );

        if let Err(e) = load_default_biomes(&mut reg, codec, &mut app.world) {
            error!("failed to load default biomes from registry codec: {e:#}");
        }
    }
}

fn update_biome_registry(
    mut reg: ResMut<BiomeRegistry>,
    mut codec: ResMut<RegistryCodec>,
    biomes: Query<(Entity, Ref<Biome>), Changed<Biome>>,
) {
    for (entity, biome) in &biomes {
        if biome.is_added() {
            add_biome_to_registry(&mut reg, biome.name.clone(), entity);
        }

        let compound = codec.get_or_insert_value(BiomeRegistry::KEY, biome.name.as_str_ident());

        *compound = compound! {
            "downfall" => biome.downfall,
            "effects" => compound! {
                "fog_color" => biome.fog_color,
                "sky_color" => biome.sky_color,
                "water_color" => biome.water_color,
                "water_fog_color" => biome.water_fog_color,
            },
            "has_precipitation" => biome.has_precipitation,
            "temperature" => biome.temperature,
        };
    }
}

fn load_default_biomes(
    reg: &mut BiomeRegistry,
    codec: &RegistryCodec,
    world: &mut World,
) -> anyhow::Result<()> {
    for value in codec.iter_values(BiomeRegistry::KEY) {
        let Value::Compound(biome) = value.element else {
            bail!("biome not a compound")
        };

        if let Value::Compound(biome) = value.element {
            let name: Ident<String> = Ident::new(value.name).context("invalid biome name")?.into();

            let name_clone = name.clone();

            let downfall = *biome
                .get("downfall")
                .and_then(|v| v.as_float())
                .context("invalid downfall")?;

            let Some(Value::Compound(effects)) = biome.get("effects") else {
                bail!("missing biome effects")
            };

            let fog_color = *effects
                .get("fog_color")
                .and_then(|v| v.as_int())
                .context("invalid fog color")?;

            let sky_color = *effects
                .get("sky_color")
                .and_then(|v| v.as_int())
                .context("invalid sky color")?;

            let water_color = *effects
                .get("water_color")
                .and_then(|v| v.as_int())
                .context("invalid water color")?;

            let water_fog_color = *effects
                .get("water_fog_color")
                .and_then(|v| v.as_int())
                .context("invalid water fog color")?;

            let has_precipitation = *biome
                .get("has_precipitation")
                .and_then(|v| v.as_byte())
                .context("invalid has_precipitation")?
                != 0;

            let temperature = *biome
                .get("temperature")
                .and_then(|v| v.as_float())
                .context("invalid temperature")?;

            let entity = world
                .spawn(Biome {
                    name,
                    downfall,
                    fog_color,
                    sky_color,
                    water_color,
                    water_fog_color,
                    has_precipitation,
                    temperature,
                })
                .id();

            add_biome_to_registry(reg, name_clone, entity);
        }
    }

    Ok(())
}

fn add_biome_to_registry(reg: &mut BiomeRegistry, name: Ident<String>, entity: Entity) {
    match reg.name_to_biome.entry(name.into_inner()) {
        Entry::Occupied(oe) => {
            warn!("biome name collision: {}", oe.key());
            oe.insert(entity);
        }
        Entry::Vacant(ve) => {
            ve.insert(entity);
        }
    }

    reg.id_to_biome.push(entity);
}

/*
impl Default for BiomeRegistry {
    fn default() -> Self {
        let mut this = Self { biomes: vec![] };

        this.add(Biome::default());
        this
    }
}

impl BiomeRegistry {
    pub const KEY: Ident<&str> = ident_str!("minecraft:worldgen/biome");

    fn new() -> Self {
        Self {
            biomes: vec![Biome::default()],
        }
    }

    pub fn get(&self, id: BiomeId) -> Option<&Biome> {
        self.biomes.get(id.0 as usize)
    }

    pub fn get_mut(&mut self, id: BiomeId) -> Option<&mut Biome> {
        self.biomes.get_mut(id.0 as usize)
    }

    pub fn add(&mut self, biome: Biome) -> BiomeId {
        self.biomes.push(biome);

        BiomeId(
            (self.biomes.len() - 1)
                .try_into()
                .expect("too many biomes added"),
        )
    }

    /// **NOTE:** This operation will invalidate all existing biome IDs!
    pub fn remove(&mut self, id: BiomeId) -> Option<Biome> {
        let idx = id.0 as usize;

        if idx < self.biomes.len() {
            Some(self.biomes.swap_remove(id.0 as _))
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (BiomeId, &Biome)> + '_ {
        self.biomes
            .iter()
            .enumerate()
            .map(|(i, b)| (BiomeId(i as _), b))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (BiomeId, &mut Biome)> + '_ {
        self.biomes
            .iter_mut()
            .enumerate()
            .map(|(i, b)| (BiomeId(i as _), b))
    }
}

impl Index<BiomeId> for BiomeRegistry {
    type Output = Biome;

    fn index(&self, id: BiomeId) -> &Self::Output {
        self.get(id).unwrap_or_else(|| panic!("invalid {id:?}"))
    }
}

impl IndexMut<BiomeId> for BiomeRegistry {
    fn index_mut(&mut self, id: BiomeId) -> &mut Self::Output {
        self.get_mut(id).unwrap_or_else(|| panic!("invalid {id:?}"))
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Biome {
    pub name: Ident<String>,
    pub downfall: f32,
    pub fog_color: i32,
    pub sky_color: i32,
    pub water_color: i32,
    pub water_fog_color: i32,
    pub has_precipitation: bool,
    pub temperature: f32,
    // TODO: more stuff.
}

impl Default for Biome {
    fn default() -> Self {
        Self {
            name: ident_str!("minecraft:plains").into(),
            downfall: 0.4,
            fog_color: 12638463,
            sky_color: 7907327,
            water_color: 4159204,
            water_fog_color: 329011,
            has_precipitation: true,
            temperature: 0.8,
        }
    }
}

impl Biome {
    fn to_registry_element(&self) -> Compound {
        compound! {
            "downfall" => self.downfall,
            "effects" => compound! {
                "fog_color" => self.fog_color,
                "sky_color" => self.sky_color,
                "water_color" => self.water_color,
                "water_fog_color" => self.water_fog_color,
            },
            "has_precipitation" => self.has_precipitation,
            "temperature" => self.temperature,
        }
    }
}

/// An index into the biome registry.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct BiomeId(pub u16);

pub(crate) struct BiomePlugin;

impl Plugin for BiomePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let mut reg = BiomeRegistry::new();

        app.insert_resource(BiomeRegistry::new()).add_system(
            update_biome_registry
                .in_base_set(CoreSet::PostUpdate)
                .before(RegistryCodecSet),
        );
    }
}

fn update_biome_registry(biomes: Res<BiomeRegistry>, mut codec: ResMut<RegistryCodec>) {
    if biomes.is_changed() {
        codec.remove_registry(BiomeRegistry::KEY);

        for (_, b) in biomes.iter() {
            _ = codec.add_element(BiomeRegistry::KEY, b.name.clone(), b.to_registry_element());
        }
    }
}
*/
*/
