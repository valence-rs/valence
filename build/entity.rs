use std::collections::BTreeMap;

use heck::ToPascalCase;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use serde::Deserialize;

use crate::ident;

#[derive(Deserialize, Clone, Debug)]
struct Entity {
    #[serde(rename = "type")]
    typ: Option<String>,
    translation_key: Option<String>,
    fields: Vec<Field>,
    parent: Option<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct EntityData {
    types: BTreeMap<String, i32>,
}

#[derive(Deserialize, Clone, Debug)]
struct Field {
    name: String,
    index: u8,
    #[serde(flatten)]
    default_value: Value,
    bits: Vec<Bit>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "default_value", rename_all = "snake_case")]
enum Value {
    Byte(u8),
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
}

#[derive(Deserialize, Debug, Clone, Copy)]
struct BlockPos {
    x: i32,
    y: i32,
    z: i32,
}

#[derive(Deserialize, Clone, Debug)]
struct Bit {
    name: String,
    index: u8,
}

impl Value {
    pub fn type_id(&self) -> i32 {
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
            Value::OptionalBlockState(_) => 14,
            Value::NbtCompound(_) => 15,
            Value::Particle(_) => 16,
            Value::VillagerData { .. } => 17,
            Value::OptionalInt(_) => 18,
            Value::EntityPose(_) => 19,
            Value::CatVariant(_) => 20,
            Value::FrogVariant(_) => 21,
            Value::OptionalGlobalPos(_) => 22,
            Value::PaintingVariant(_) => 23,
        }
    }

    pub fn field_type(&self) -> TokenStream {
        match self {
            Value::Byte(_) => quote!(u8),
            Value::Integer(_) => quote!(i32),
            Value::Long(_) => quote!(i64),
            Value::Float(_) => quote!(f32),
            Value::String(_) => quote!(Box<str>),
            Value::TextComponent(_) => quote!(Text),
            Value::OptionalTextComponent(_) => quote!(Option<Text>),
            Value::ItemStack(_) => quote!(()), // TODO
            Value::Boolean(_) => quote!(bool),
            Value::Rotation { .. } => quote!(EulerAngle),
            Value::BlockPos(_) => quote!(BlockPos),
            Value::OptionalBlockPos(_) => quote!(Option<BlockPos>),
            Value::Facing(_) => quote!(Facing),
            Value::OptionalUuid(_) => quote!(Option<Uuid>),
            Value::OptionalBlockState(_) => quote!(BlockState),
            Value::NbtCompound(_) => quote!(crate::nbt::Compound),
            Value::Particle(_) => quote!(Particle),
            Value::VillagerData { .. } => quote!(VillagerData),
            Value::OptionalInt(_) => quote!(OptionalInt),
            Value::EntityPose(_) => quote!(Pose),
            Value::CatVariant(_) => quote!(CatKind),
            Value::FrogVariant(_) => quote!(FrogKind),
            Value::OptionalGlobalPos(_) => quote!(()), // TODO
            Value::PaintingVariant(_) => quote!(PaintingKind),
        }
    }

    pub fn getter_return_type(&self) -> TokenStream {
        match self {
            Value::String(_) => quote!(&str),
            Value::TextComponent(_) => quote!(&Text),
            Value::OptionalTextComponent(_) => quote!(Option<&Text>),
            Value::NbtCompound(_) => quote!(&crate::nbt::Compound),
            _ => self.field_type(),
        }
    }

    pub fn getter_return_expr(&self, field_name: &Ident) -> TokenStream {
        match self {
            Value::String(_) | Value::TextComponent(_) | Value::NbtCompound(_) => {
                quote!(&self.#field_name)
            }
            Value::OptionalTextComponent(_) => quote!(self.#field_name.as_ref()),
            _ => quote!(self.#field_name),
        }
    }

    pub fn default_expr(&self) -> TokenStream {
        match self {
            Value::Byte(b) => quote!(#b),
            Value::Integer(i) => quote!(#i),
            Value::Long(l) => quote!(#l),
            Value::Float(f) => quote!(#f),
            Value::String(s) => quote!(#s.to_owned().into_boxed_str()),
            Value::TextComponent(_) => quote!(Text::default()), // TODO
            Value::OptionalTextComponent(t) => {
                assert!(t.is_none());
                quote!(None)
            }
            Value::ItemStack(_) => quote!(()), // TODO
            Value::Boolean(b) => quote!(#b),
            Value::Rotation { pitch, yaw, roll } => quote! {
                EulerAngle {
                    pitch: #pitch,
                    yaw: #yaw,
                    roll: #roll,
                }
            },
            Value::BlockPos(BlockPos { x, y, z }) => {
                quote!(BlockPos { x: #x, y: #y, z: #z })
            }
            Value::OptionalBlockPos(_) => quote!(None), // TODO
            Value::Facing(f) => {
                let variant = ident(f.to_pascal_case());
                quote!(Facing::#variant)
            }
            Value::OptionalUuid(_) => quote!(None), // TODO
            Value::OptionalBlockState(_) => quote!(BlockState::default()), // TODO
            Value::NbtCompound(_) => quote!(crate::nbt::Compound::default()), // TODO
            Value::Particle(p) => {
                let variant = ident(p.to_pascal_case());
                quote!(Particle::#variant)
            }
            Value::VillagerData {
                typ,
                profession,
                level,
            } => {
                let typ = ident(typ.to_pascal_case());
                let profession = ident(profession.to_pascal_case());
                quote!(VillagerData::new(VillagerKind::#typ, VillagerProfession::#profession, #level))
            }
            Value::OptionalInt(i) => {
                assert!(i.is_none());
                quote!(OptionalInt::default())
            }
            Value::EntityPose(p) => {
                let variant = ident(p.to_pascal_case());
                quote!(Pose::#variant)
            }
            Value::CatVariant(c) => {
                let variant = ident(c.to_pascal_case());
                quote!(CatKind::#variant)
            }
            Value::FrogVariant(f) => {
                let variant = ident(f.to_pascal_case());
                quote!(FrogKind::#variant)
            }
            Value::OptionalGlobalPos(_) => quote!(()),
            Value::PaintingVariant(p) => {
                let variant = ident(p.to_pascal_case());
                quote!(PaintingKind::#variant)
            }
        }
    }

    pub fn encodable_expr(&self, self_lvalue: TokenStream) -> TokenStream {
        match self {
            Value::Integer(_) => quote!(VarInt(#self_lvalue)),
            _ => self_lvalue,
        }
    }
}

type Entities = BTreeMap<String, Entity>;

pub fn build() -> anyhow::Result<TokenStream> {
    let entities = serde_json::from_str::<Entities>(include_str!("../extracted/entities.json"))?
        .into_iter()
        .map(|(k, mut v)| {
            let strip = |s: String| {
                if let Some(stripped) = s.strip_suffix("Entity") {
                    if !stripped.is_empty() {
                        return stripped.to_owned();
                    }
                }
                s
            };
            v.parent = v.parent.map(strip);
            (strip(k), v)
        })
        .collect::<Entities>();

    let entity_types =
        serde_json::from_str::<EntityData>(include_str!("../extracted/entity_data.json"))?.types;

    let concrete_entities = entities
        .clone()
        .into_iter()
        .filter(|(_, v)| v.typ.is_some())
        .collect::<Entities>();

    let entity_kind_variants = concrete_entities.iter().map(|(name, e)| {
        let name = ident(name);
        let id = entity_types[e.typ.as_ref().unwrap()] as isize;
        quote! {
            #name = #id,
        }
    });

    let concrete_entity_names = concrete_entities.keys().map(ident).collect::<Vec<_>>();

    let concrete_entity_structs = concrete_entities.keys().map(|struct_name| {
        let fields = collect_all_fields(struct_name, &entities);
        let struct_name = ident(struct_name);

        let modified_flags_type =
            ident("u".to_owned() + &fields.len().next_power_of_two().max(8).to_string());

        let struct_fields = fields.iter().map(|&field| {
            let name = ident(&field.name);
            let typ = field.default_value.field_type();
            quote! {
                #name: #typ,
            }
        });

        let field_initializers = fields.iter().map(|&field| {
            let field_name = ident(&field.name);
            let init = field.default_value.default_expr();

            quote! {
                #field_name: #init,
            }
        });

        let getter_setters = fields.iter().map(|&field| {
            let field_name = ident(&field.name);
            let field_type = field.default_value.field_type();
            let field_index = field.index;

            if !field.bits.is_empty() {
                field
                    .bits
                    .iter()
                    .map(|bit| {
                        let bit_name = ident(&bit.name);
                        let bit_index = bit.index;
                        let getter_name = ident(format!("get_{}", &bit.name));
                        let setter_name = ident(format!("set_{}", &bit.name));

                        quote! {
                            pub fn #getter_name(&self) -> bool {
                                self.#field_name >> #bit_index as #field_type & 1 == 1
                            }

                            pub fn #setter_name(&mut self, #bit_name: bool) {
                                if self.#getter_name() != #bit_name {
                                    self.#field_name =
                                        (self.#field_name & !(1 << #bit_index as #field_type))
                                        | ((#bit_name as #field_type) << #bit_index);

                                    self.__modified_flags |= 1 << #field_index
                                }
                            }
                        }
                    })
                    .collect::<TokenStream>()
            } else {
                let getter_name = ident(format!("get_{}", &field.name));
                let setter_name = ident(format!("set_{}", &field.name));
                let getter_return_type = field.default_value.getter_return_type();
                let getter_return_expr = field.default_value.getter_return_expr(&field_name);

                quote! {
                    pub fn #getter_name(&self) -> #getter_return_type {
                        #getter_return_expr
                    }

                    pub fn #setter_name(&mut self, #field_name: impl Into<#field_type>) {
                        let #field_name = #field_name.into();
                        if self.#field_name != #field_name {
                            self.__modified_flags |= 1 << #field_index as #modified_flags_type;
                            self.#field_name = #field_name;
                        }
                    }
                }
            }
        });

        let initial_tracked_data_stmts = fields.iter().map(|&field| {
            let field_name = ident(&field.name);
            let field_index = field.index;
            let default_expr = field.default_value.default_expr();
            let type_id = field.default_value.type_id();
            let encodable = field.default_value.encodable_expr(quote!(self.#field_name));

            quote! {
                if self.#field_name != (#default_expr) {
                    data.push(#field_index);
                    VarInt(#type_id).encode(&mut *data).unwrap();
                    #encodable.encode(&mut *data).unwrap();
                }
            }
        });

        let updated_tracked_data_stmts = fields.iter().map(|&field| {
            let field_name = ident(&field.name);
            let field_index = field.index;
            let type_id = field.default_value.type_id();
            let encodable = field.default_value.encodable_expr(quote!(self.#field_name));

            quote! {
                if (self.__modified_flags >> #field_index as #modified_flags_type) & 1 == 1 {
                    data.push(#field_index);
                    VarInt(#type_id).encode(&mut *data).unwrap();
                    #encodable.encode(&mut *data).unwrap();
                }
            }
        });

        quote! {
            pub struct #struct_name {
                /// Contains a set bit for every modified field.
                __modified_flags: #modified_flags_type,
                #(#struct_fields)*
            }

            impl #struct_name {
                pub(crate) fn new() -> Self {
                    Self {
                        __modified_flags: 0,
                        #(#field_initializers)*
                    }
                }

                pub(crate) fn initial_tracked_data(&self, data: &mut Vec<u8>) {
                    #(#initial_tracked_data_stmts)*
                }

                pub(crate) fn updated_tracked_data(&self, data: &mut Vec<u8>) {
                    if self.__modified_flags != 0 {
                        #(#updated_tracked_data_stmts)*
                    }
                }

                pub(crate) fn clear_modifications(&mut self) {
                    self.__modified_flags = 0;
                }

                #(#getter_setters)*
            }
        }
    });

    let translation_key_arms = concrete_entities.iter().map(|(k, v)| {
        let name = ident(k);
        let key = v
            .translation_key
            .as_ref()
            .expect("translation key should be present for concrete entity");

        quote! {
            Self::#name => #key,
        }
    });

    Ok(quote! {
        /// Contains a variant for each concrete entity type.
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum EntityKind {
            #(#entity_kind_variants)*
        }

        impl EntityKind {
            pub fn translation_key(self) -> &'static str {
                match self {
                    #(#translation_key_arms)*
                }
            }
        }

        pub enum TrackedData {
            #(#concrete_entity_names(#concrete_entity_names),)*
        }

        impl TrackedData {
            pub(super) fn new(kind: EntityKind) -> Self {
                match kind {
                    #(EntityKind::#concrete_entity_names => Self::#concrete_entity_names(#concrete_entity_names::new()),)*
                }
            }

            pub fn kind(&self) -> EntityKind {
                match self {
                    #(Self::#concrete_entity_names(_) => EntityKind::#concrete_entity_names,)*
                }
            }

            pub(super) fn write_initial_tracked_data(&self, buf: &mut Vec<u8>) {
                buf.clear();

                match self {
                    #(Self::#concrete_entity_names(e) => e.initial_tracked_data(buf),)*
                }

                if !buf.is_empty() {
                    buf.push(0xff);
                }
            }

            pub(super) fn write_updated_tracked_data(&self, buf: &mut Vec<u8>) {
                buf.clear();

                match self {
                    #(Self::#concrete_entity_names(e) => e.updated_tracked_data(buf),)*
                }

                if !buf.is_empty() {
                    buf.push(0xff);
                }
            }

            pub(super) fn clear_modifications(&mut self) {
                match self {
                    #(Self::#concrete_entity_names(e) => e.clear_modifications(),)*
                }
            }
        }

        #(#concrete_entity_structs)*
    })
}

fn collect_all_fields<'a>(entity_name: &str, entities: &'a Entities) -> Vec<&'a Field> {
    fn rec<'a>(entity_name: &str, entities: &'a Entities, fields: &mut Vec<&'a Field>) {
        let e = &entities[entity_name];
        fields.extend(&e.fields);

        if let Some(parent) = &e.parent {
            rec(parent, entities, fields);
        }
    }

    let mut fields = vec![];
    rec(entity_name, entities, &mut fields);

    fields.sort_by_key(|f| f.index);

    fields
}
