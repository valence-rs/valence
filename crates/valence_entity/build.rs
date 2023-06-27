use std::collections::BTreeMap;

use anyhow::Context;
use heck::{ToPascalCase, ToShoutySnakeCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use valence_build_utils::{ident, rerun_if_changed, write_generated_file};

#[derive(Deserialize, Clone, Debug)]
struct Entity {
    #[serde(rename = "type")]
    typ: Option<String>,
    translation_key: Option<String>,
    fields: Vec<Field>,
    parent: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct EntityTypes {
    entity_type: BTreeMap<String, i32>,
}

#[derive(Deserialize, Clone, Debug)]
struct Field {
    name: String,
    index: u8,
    #[serde(flatten)]
    default_value: Value,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "default_value", rename_all = "snake_case")]
enum Value {
    Byte(i8),
    Integer(i32),
    Long(i64),
    Float(f32),
    String(String),
    TextComponent(String),
    OptionalTextComponent(Option<String>),
    ItemStack(String),
    Boolean(bool),
    Rotation {
        pitch: f32,
        yaw: f32,
        roll: f32,
    },
    BlockPos(BlockPos),
    OptionalBlockPos(Option<BlockPos>),
    Facing(String),
    OptionalUuid(Option<String>),
    BlockState(String),
    OptionalBlockState(Option<String>),
    NbtCompound(String),
    Particle(String),
    VillagerData {
        #[serde(rename = "type")]
        typ: String,
        profession: String,
        level: i32,
    },
    OptionalInt(Option<i32>),
    EntityPose(String),
    CatVariant(String),
    FrogVariant(String),
    OptionalGlobalPos(Option<()>), // TODO
    PaintingVariant(String),
    SnifferState(String),
    Vector3f {
        x: f32,
        y: f32,
        z: f32,
    },
    Quaternionf {
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    },
}

#[derive(Deserialize, Debug, Clone, Copy)]
struct BlockPos {
    x: i32,
    y: i32,
    z: i32,
}

impl Value {
    pub fn type_id(&self) -> u8 {
        match self {
            Value::Byte(_) => 0,
            Value::Integer(_) => 1,
            Value::Long(_) => 2,
            Value::Float(_) => 3,
            Value::String(_) => 4,
            Value::TextComponent(_) => 5,
            Value::OptionalTextComponent(_) => 6,
            Value::ItemStack(_) => 7,
            Value::Boolean(_) => 8,
            Value::Rotation { .. } => 9,
            Value::BlockPos(_) => 10,
            Value::OptionalBlockPos(_) => 11,
            Value::Facing(_) => 12,
            Value::OptionalUuid(_) => 13,
            Value::BlockState(_) => 14,
            Value::OptionalBlockState(_) => 15,
            Value::NbtCompound(_) => 16,
            Value::Particle(_) => 17,
            Value::VillagerData { .. } => 18,
            Value::OptionalInt(_) => 19,
            Value::EntityPose(_) => 20,
            Value::CatVariant(_) => 21,
            Value::FrogVariant(_) => 22,
            Value::OptionalGlobalPos(_) => 23,
            Value::PaintingVariant(_) => 24,
            Value::SnifferState(_) => 25,
            Value::Vector3f { .. } => 26,
            Value::Quaternionf { .. } => 27,
        }
    }

    pub fn field_type(&self) -> TokenStream {
        match self {
            Value::Byte(_) => quote!(i8),
            Value::Integer(_) => quote!(i32),
            Value::Long(_) => quote!(i64),
            Value::Float(_) => quote!(f32),
            Value::String(_) => quote!(String),
            Value::TextComponent(_) => quote!(valence_core::text::Text),
            Value::OptionalTextComponent(_) => quote!(Option<valence_core::text::Text>),
            Value::ItemStack(_) => quote!(valence_core::item::ItemStack),
            Value::Boolean(_) => quote!(bool),
            Value::Rotation { .. } => quote!(crate::EulerAngle),
            Value::BlockPos(_) => quote!(valence_core::block_pos::BlockPos),
            Value::OptionalBlockPos(_) => quote!(Option<valence_core::block_pos::BlockPos>),
            Value::Facing(_) => quote!(valence_core::direction::Direction),
            Value::OptionalUuid(_) => quote!(Option<::uuid::Uuid>),
            Value::BlockState(_) => quote!(valence_block::BlockState),
            Value::OptionalBlockState(_) => quote!(valence_block::BlockState),
            Value::NbtCompound(_) => quote!(valence_nbt::Compound),
            Value::Particle(_) => quote!(valence_core::particle::Particle),
            Value::VillagerData { .. } => quote!(crate::VillagerData),
            Value::OptionalInt(_) => quote!(Option<i32>),
            Value::EntityPose(_) => quote!(crate::Pose),
            Value::CatVariant(_) => quote!(crate::CatKind),
            Value::FrogVariant(_) => quote!(crate::FrogKind),
            Value::OptionalGlobalPos(_) => quote!(()), // TODO
            Value::PaintingVariant(_) => quote!(crate::PaintingKind),
            Value::SnifferState(_) => quote!(crate::SnifferState),
            Value::Vector3f { .. } => quote!(glam::f32::Vec3),
            Value::Quaternionf { .. } => quote!(glam::f32::Quat),
        }
    }

    pub fn default_expr(&self) -> TokenStream {
        match self {
            Value::Byte(b) => quote!(#b),
            Value::Integer(i) => quote!(#i),
            Value::Long(l) => quote!(#l),
            Value::Float(f) => quote!(#f),
            Value::String(s) => quote!(#s.to_owned()),
            Value::TextComponent(txt) => {
                assert!(txt.is_empty());
                quote!(valence_core::text::Text::default())
            }
            Value::OptionalTextComponent(t) => {
                assert!(t.is_none());
                quote!(None)
            }
            Value::ItemStack(stack) => {
                assert_eq!(stack, "0 air");
                quote!(valence_core::item::ItemStack::default())
            }
            Value::Boolean(b) => quote!(#b),
            Value::Rotation { pitch, yaw, roll } => quote! {
                crate::EulerAngle {
                    pitch: #pitch,
                    yaw: #yaw,
                    roll: #roll,
                }
            },
            Value::BlockPos(BlockPos { x, y, z }) => {
                quote!(valence_core::block_pos::BlockPos { x: #x, y: #y, z: #z })
            }
            Value::OptionalBlockPos(pos) => {
                assert!(pos.is_none());
                quote!(None)
            }
            Value::Facing(f) => {
                let variant = ident(f.replace('.', "_").to_pascal_case());
                quote!(valence_core::direction::Direction::#variant)
            }
            Value::OptionalUuid(uuid) => {
                assert!(uuid.is_none());
                quote!(None)
            }
            Value::BlockState(_) => {
                quote!(valence_block::BlockState::default())
            }
            Value::OptionalBlockState(bs) => {
                assert!(bs.is_none());
                quote!(valence_block::BlockState::default())
            }
            Value::NbtCompound(s) => {
                assert_eq!(s, "{}");
                quote!(valence_nbt::Compound::default())
            }
            Value::Particle(p) => {
                let variant = ident(p.replace('.', "_").to_pascal_case());
                quote!(valence_core::particle::Particle::#variant)
            }
            Value::VillagerData {
                typ,
                profession,
                level,
            } => {
                let typ = ident(typ.replace('.', "_").to_pascal_case());
                let profession = ident(profession.replace('.', "_").to_pascal_case());
                quote! {
                    crate::VillagerData {
                        kind: crate::VillagerKind::#typ,
                        profession: crate::VillagerProfession::#profession,
                        level: #level,
                    }
                }
            }
            Value::OptionalInt(i) => {
                assert!(i.is_none());
                quote!(None)
            }
            Value::EntityPose(p) => {
                let variant = ident(p.replace('.', "_").to_pascal_case());
                quote!(crate::Pose::#variant)
            }
            Value::CatVariant(c) => {
                let variant = ident(c.replace('.', "_").to_pascal_case());
                quote!(crate::CatKind::#variant)
            }
            Value::FrogVariant(f) => {
                let variant = ident(f.replace('.', "_").to_pascal_case());
                quote!(crate::FrogKind::#variant)
            }
            Value::OptionalGlobalPos(_) => quote!(()),
            Value::PaintingVariant(p) => {
                let variant = ident(p.replace('.', "_").to_pascal_case());
                quote!(crate::PaintingKind::#variant)
            }
            Value::SnifferState(s) => {
                let state = ident(s.replace('.', "_").to_pascal_case());
                quote!(crate::SnifferState::#state)
            }
            Value::Vector3f { x, y, z } => quote!(glam::f32::Vec3::new(#x, #y, #z)),
            Value::Quaternionf { x, y, z, w } => quote! {
                glam::f32::Quat::from_xyzw(#x, #y, #z, #w)
            },
        }
    }

    pub fn encodable_expr(&self, self_lvalue: TokenStream) -> TokenStream {
        match self {
            Value::Integer(_) => quote!(VarInt(#self_lvalue)),
            Value::OptionalInt(_) => quote!(OptionalInt(#self_lvalue)),
            Value::ItemStack(_) => quote!(Some(&#self_lvalue)),
            _ => quote!(&#self_lvalue),
        }
    }
}

type Entities = BTreeMap<String, Entity>;

pub fn main() -> anyhow::Result<()> {
    rerun_if_changed(["../../extracted/misc.json", "../../extracted/entities.json"]);

    write_generated_file(build()?, "entity.rs")
}

fn build() -> anyhow::Result<TokenStream> {
    let entity_types =
        serde_json::from_str::<EntityTypes>(include_str!("../../extracted/misc.json"))
            .context("failed to deserialize misc.json")?
            .entity_type;

    let entities: Entities =
        serde_json::from_str::<Entities>(include_str!("../../extracted/entities.json"))
            .context("failed to deserialize entities.json")?
            .into_iter()
            .collect();

    let mut entity_kind_consts = TokenStream::new();
    let mut entity_kind_fmt_args = TokenStream::new();
    let mut translation_key_arms = TokenStream::new();
    let mut modules = TokenStream::new();
    let mut systems = TokenStream::new();
    let mut system_names = vec![];

    for (entity_name, entity) in entities.clone() {
        let entity_name_ident = ident(&entity_name);
        let stripped_shouty_entity_name = strip_entity_suffix(&entity_name)
            .replace('.', "_")
            .to_shouty_snake_case();
        let stripped_shouty_entity_name_ident = ident(&stripped_shouty_entity_name);
        let stripped_snake_entity_name = strip_entity_suffix(&entity_name).to_snake_case();
        let stripped_snake_entity_name_ident = ident(&stripped_snake_entity_name);

        let mut module_body = TokenStream::new();

        if let Some(parent_name) = entity.parent {
            let stripped_snake_parent_name = strip_entity_suffix(&parent_name).to_snake_case();

            let module_doc = format!(
                "Parent class: \
                 [`{stripped_snake_parent_name}`][super::{stripped_snake_parent_name}]."
            );

            module_body.extend([quote! {
                #![doc = #module_doc]
            }]);
        }

        // Is this a concrete entity type?
        if let Some(entity_type) = entity.typ {
            let entity_type_id = entity_types[&entity_type];

            entity_kind_consts.extend([quote! {
                pub const #stripped_shouty_entity_name_ident: EntityKind = EntityKind(#entity_type_id);
            }]);

            entity_kind_fmt_args.extend([quote! {
                EntityKind::#stripped_shouty_entity_name_ident => write!(f, "{} ({})", #entity_type_id, #stripped_shouty_entity_name),
            }]);

            let translation_key_expr = if let Some(key) = entity.translation_key {
                quote!(Some(#key))
            } else {
                quote!(None)
            };

            translation_key_arms.extend([quote! {
                EntityKind::#stripped_shouty_entity_name_ident => #translation_key_expr,
            }]);

            // Create bundle type.
            let mut bundle_fields = TokenStream::new();
            let mut bundle_init_fields = TokenStream::new();

            for marker_or_field in collect_bundle_fields(&entity_name, &entities) {
                match marker_or_field {
                    MarkerOrField::Marker { entity_name } => {
                        let stripped_entity_name = strip_entity_suffix(entity_name);

                        let snake_entity_name_ident = ident(entity_name.to_snake_case());
                        let stripped_snake_entity_name_ident =
                            ident(stripped_entity_name.to_snake_case());
                        let pascal_entity_name_ident =
                            ident(entity_name.replace('.', "_").to_pascal_case());

                        bundle_fields.extend([quote! {
                            pub #snake_entity_name_ident: super::#stripped_snake_entity_name_ident::#pascal_entity_name_ident,
                        }]);

                        bundle_init_fields.extend([quote! {
                            #snake_entity_name_ident: Default::default(),
                        }]);
                    }
                    MarkerOrField::Field { entity_name, field } => {
                        let snake_field_name = field.name.to_snake_case();
                        let pascal_field_name = field.name.replace('.', "_").to_pascal_case();
                        let pascal_field_name_ident = ident(&pascal_field_name);
                        let stripped_entity_name = strip_entity_suffix(entity_name);
                        let stripped_snake_entity_name = stripped_entity_name.to_snake_case();
                        let stripped_snake_entity_name_ident = ident(&stripped_snake_entity_name);

                        let field_name_ident =
                            ident(format!("{stripped_snake_entity_name}_{snake_field_name}"));

                        bundle_fields.extend([quote! {
                            pub #field_name_ident: super::#stripped_snake_entity_name_ident::#pascal_field_name_ident,
                        }]);

                        bundle_init_fields.extend([quote! {
                            #field_name_ident: Default::default(),
                        }]);
                    }
                }
            }

            bundle_fields.extend([quote! {
                pub kind: super::EntityKind,
                pub id: super::EntityId,
                pub uuid: super::UniqueId,
                pub location: super::Location,
                pub old_location: super::OldLocation,
                pub position: super::Position,
                pub old_position: super::OldPosition,
                pub look: super::Look,
                pub head_yaw: super::HeadYaw,
                pub on_ground: super::OnGround,
                pub velocity: super::Velocity,
                pub statuses: super::EntityStatuses,
                pub animations: super::EntityAnimations,
                pub object_data: super::ObjectData,
                pub tracked_data: super::TrackedData,
            }]);

            bundle_init_fields.extend([quote! {
                kind: super::EntityKind::#stripped_shouty_entity_name_ident,
                id: Default::default(),
                uuid: Default::default(),
                location: Default::default(),
                old_location: Default::default(),
                position: Default::default(),
                old_position: Default::default(),
                look: Default::default(),
                head_yaw: Default::default(),
                on_ground: Default::default(),
                velocity: Default::default(),
                statuses: Default::default(),
                animations: Default::default(),
                object_data: Default::default(),
                tracked_data: Default::default(),
            }]);

            let bundle_name_ident = ident(format!("{entity_name}Bundle"));
            let bundle_doc = format!(
                "The bundle of components for spawning `{stripped_snake_entity_name}` entities."
            );

            module_body.extend([quote! {
                #[doc = #bundle_doc]
                #[derive(bevy_ecs::bundle::Bundle, Debug)]
                pub struct #bundle_name_ident {
                    #bundle_fields
                }

                impl Default for #bundle_name_ident {
                    fn default() -> Self {
                        Self {
                            #bundle_init_fields
                        }
                    }
                }
            }]);
        }

        for field in &entity.fields {
            let pascal_field_name_ident = ident(field.name.replace('.', "_").to_pascal_case());
            let snake_field_name = field.name.to_snake_case();
            let inner_type = field.default_value.field_type();
            let default_expr = field.default_value.default_expr();

            module_body.extend([quote! {
                #[derive(bevy_ecs::component::Component, PartialEq, Clone, Debug)]
                pub struct #pascal_field_name_ident(pub #inner_type);

                #[allow(clippy::derivable_impls)]
                impl Default for #pascal_field_name_ident {
                    fn default() -> Self {
                        Self(#default_expr)
                    }
                }
            }]);

            let system_name_ident = ident(format!(
                "update_{stripped_snake_entity_name}_{snake_field_name}"
            ));
            let component_path =
                quote!(#stripped_snake_entity_name_ident::#pascal_field_name_ident);

            system_names.push(quote!(#system_name_ident));

            let data_index = field.index;
            let data_type = field.default_value.type_id();
            let encodable_expr = field.default_value.encodable_expr(quote!(value.0));

            systems.extend([quote! {
                #[allow(clippy::needless_borrow)]
                fn #system_name_ident(
                    mut query: Query<(&#component_path, &mut TrackedData), Changed<#component_path>>
                ) {
                    for (value, mut tracked_data) in &mut query {
                        if *value == Default::default() {
                            tracked_data.remove_init_value(#data_index);
                        } else {
                            tracked_data.insert_init_value(#data_index, #data_type, #encodable_expr);
                        }

                        if !tracked_data.is_added() {
                            tracked_data.append_update_value(#data_index, #data_type, #encodable_expr);
                        }
                    }
                }
            }]);
        }

        let marker_doc = format!("Marker component for `{stripped_snake_entity_name}` entities.");

        module_body.extend([quote! {
            #[doc = #marker_doc]
            #[derive(bevy_ecs::component::Component, Copy, Clone, Default, Debug)]
            pub struct #entity_name_ident;
        }]);

        modules.extend([quote! {
            #[allow(clippy::module_inception)]
            pub mod #stripped_snake_entity_name_ident {
                #module_body
            }
        }]);
    }

    #[derive(Deserialize, Debug)]
    struct MiscEntityData {
        entity_status: BTreeMap<String, u8>,
        entity_animation: BTreeMap<String, u8>,
    }

    let misc_entity_data: MiscEntityData =
        serde_json::from_str(include_str!("../../extracted/misc.json"))?;

    let entity_status_variants = misc_entity_data
        .entity_status
        .into_iter()
        .map(|(name, code)| {
            let name = ident(name.replace('.', "_").to_pascal_case());
            let code = code as isize;

            quote! {
                #name = #code,
            }
        });

    let entity_animation_variants =
        misc_entity_data
            .entity_animation
            .into_iter()
            .map(|(name, code)| {
                let name = ident(name.replace('.', "_").to_pascal_case());
                let code = code as isize;

                quote! {
                    #name = #code,
                }
            });

    Ok(quote! {
        #modules

        /// Identifies the type of an entity.
        /// As a component, the entity kind should not be modified.
        #[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct EntityKind(i32);

        impl EntityKind {
            #entity_kind_consts

            pub const fn new(inner: i32) -> Self {
                Self(inner)
            }

            pub const fn get(self) -> i32 {
                self.0
            }

            pub const fn translation_key(self) -> Option<&'static str> {
                match self {
                    #translation_key_arms
                    _ => None,
                }
            }
        }

        impl std::fmt::Debug for EntityKind {
            #[allow(clippy::write_literal)]
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match *self {
                    #entity_kind_fmt_args
                    EntityKind(other) => write!(f, "{other}"),
                }
            }
        }

        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum EntityStatus {
            #(#entity_status_variants)*
        }

        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum EntityAnimation {
            #(#entity_animation_variants)*
        }

        fn add_tracked_data_systems(app: &mut App) {
            #systems

            #(
                app.add_systems(
                    PostUpdate,
                    #system_names
                        .in_set(UpdateTrackedDataSet)
                        .ambiguous_with(UpdateTrackedDataSet)
                );
            )*
        }
    })
}

enum MarkerOrField<'a> {
    Marker {
        entity_name: &'a str,
    },
    Field {
        entity_name: &'a str,
        field: &'a Field,
    },
}

fn collect_bundle_fields<'a>(
    mut entity_name: &'a str,
    entities: &'a Entities,
) -> Vec<MarkerOrField<'a>> {
    let mut res = vec![];

    loop {
        let e = &entities[entity_name];

        res.push(MarkerOrField::Marker { entity_name });
        res.extend(
            e.fields
                .iter()
                .map(|field| MarkerOrField::Field { entity_name, field }),
        );

        if let Some(parent) = &e.parent {
            entity_name = parent;
        } else {
            break;
        }
    }

    res
}

fn strip_entity_suffix(string: &str) -> String {
    let stripped = string.strip_suffix("Entity").unwrap_or(string);

    if stripped.is_empty() {
        string
    } else {
        stripped
    }
    .to_owned()
}
