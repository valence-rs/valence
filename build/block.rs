use std::collections::BTreeSet;

use heck::{ToPascalCase, ToShoutySnakeCase};
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

use crate::ident;
use crate::item::Item;

#[derive(Deserialize, Clone, Debug)]
struct TopLevel {
    blocks: Vec<Block>,
    shapes: Vec<Shape>,
}

#[derive(Deserialize, Clone, Debug)]
struct Block {
    #[allow(unused)]
    id: u16,
    item_id: u16,
    translation_key: String,
    name: String,
    properties: Vec<Property>,
    default_state_id: u16,
    states: Vec<State>,
}

impl Block {
    pub fn min_state_id(&self) -> u16 {
        self.states.iter().map(|s| s.id).min().unwrap()
    }

    pub fn max_state_id(&self) -> u16 {
        self.states.iter().map(|s| s.id).max().unwrap()
    }
}

#[derive(Deserialize, Clone, Debug)]
struct Property {
    name: String,
    values: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
struct State {
    id: u16,
    luminance: u8,
    opaque: bool,
    collision_shapes: Vec<u16>,
}

#[derive(Deserialize, Clone, Debug)]
struct Shape {
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
}

pub fn build() -> anyhow::Result<TokenStream> {
    let TopLevel { blocks, shapes } =
        serde_json::from_str(include_str!("../extracted/blocks.json"))?;

    let max_state_id = blocks.iter().map(|b| b.max_state_id()).max().unwrap();

    let kind_to_translation_key_arms = blocks
        .iter()
        .map(|b| {
            let kind = ident(b.name.to_pascal_case());
            let translation_key = &b.translation_key;
            quote! {
                Self::#kind => #translation_key,
            }
        })
        .collect::<TokenStream>();

    let state_to_kind_arms = blocks
        .iter()
        .map(|b| {
            let min = b.min_state_id();
            let max = b.max_state_id();
            let name = ident(&b.name.to_pascal_case());
            quote! {
                #min..=#max => BlockKind::#name,
            }
        })
        .collect::<TokenStream>();

    let state_to_luminance_arms = blocks
        .iter()
        .flat_map(|b| {
            b.states.iter().filter(|s| s.luminance != 0).map(|s| {
                let id = s.id;
                let luminance = s.luminance;
                quote! {
                    #id => #luminance,
                }
            })
        })
        .collect::<TokenStream>();

    let state_to_opaque_arms = blocks
        .iter()
        .flat_map(|b| {
            b.states.iter().filter(|s| !s.opaque).map(|s| {
                let id = s.id;
                quote! {
                    #id => false,
                }
            })
        })
        .collect::<TokenStream>();

    let shapes = shapes.iter().map(|s| {
        let min_x = s.min_x;
        let min_y = s.min_y;
        let min_z = s.min_z;
        let max_x = s.max_x;
        let max_y = s.max_y;
        let max_z = s.max_z;
        quote! {
            [
                #min_x,
                #min_y,
                #min_z,
                #max_x,
                #max_y,
                #max_z,
            ]
        }
    });

    let shape_count = shapes.len();

    let state_to_collision_shapes_arms = blocks
        .iter()
        .flat_map(|b| {
            b.states.iter().map(|s| {
                let id = s.id;
                let collision_shapes = &s.collision_shapes;
                quote! {
                    #id => &[#(#collision_shapes),*],
                }
            })
        })
        .collect::<TokenStream>();

    let get_arms = blocks
        .iter()
        .filter(|&b| !b.properties.is_empty())
        .map(|b| {
            let block_kind_name = ident(b.name.to_pascal_case());

            let arms = b
                .properties
                .iter()
                .map(|p| {
                    let prop_name = ident(p.name.to_pascal_case());
                    let min_state_id = b.min_state_id();
                    let product: u16 = b
                        .properties
                        .iter()
                        .rev()
                        .take_while(|&other| p.name != other.name)
                        .map(|p| p.values.len() as u16)
                        .product();

                    let values_count = p.values.len() as u16;

                    let arms = p.values.iter().enumerate().map(|(i, v)| {
                        let value_idx = i as u16;
                        let value_name = ident(v.to_pascal_case());
                        quote! {
                            #value_idx => Some(PropValue::#value_name),
                        }
                    }).collect::<TokenStream>();

                    quote! {
                        PropName::#prop_name => match (self.0 - #min_state_id) / #product % #values_count {
                            #arms
                            _ => unreachable!(),
                        },
                    }
                })
                .collect::<TokenStream>();

            quote! {
                BlockKind::#block_kind_name => match name {
                    #arms
                    _ => None,
                },
            }
        })
        .collect::<TokenStream>();

    let set_arms = blocks
        .iter()
        .filter(|&b| !b.properties.is_empty())
        .map(|b| {
            let block_kind_name = ident(b.name.to_pascal_case());

            let arms = b
                .properties
                .iter()
                .map(|p| {
                    let prop_name = ident(p.name.to_pascal_case());
                    let min_state_id = b.min_state_id();
                    let product: u16 = b
                        .properties
                        .iter()
                        .rev()
                        .take_while(|&other| p.name != other.name)
                        .map(|p| p.values.len() as u16)
                        .product();

                    let values_count = p.values.len() as u16;

                    let arms = p
                        .values
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let val_idx = i as u16;
                            let val_name = ident(v.to_pascal_case());
                            quote! {
                                PropValue::#val_name =>
                                    Self(self.0 - (self.0 - #min_state_id) / #product % #values_count * #product
                                        + #val_idx * #product),
                            }
                        })
                        .collect::<TokenStream>();

                    quote! {
                        PropName::#prop_name => match val {
                            #arms
                            _ => self,
                        },
                    }
                })
                .collect::<TokenStream>();

            quote! {
                BlockKind::#block_kind_name => match name {
                    #arms
                    _ => self,
                },
            }
        })
        .collect::<TokenStream>();

    let default_block_states = blocks
        .iter()
        .map(|b| {
            let name = ident(b.name.to_shouty_snake_case());
            let state = b.default_state_id;
            let doc = format!("The default block state for `{}`.", b.name);
            quote! {
                #[doc = #doc]
                pub const #name: BlockState = BlockState(#state);
            }
        })
        .collect::<TokenStream>();

    let kind_to_state_arms = blocks
        .iter()
        .map(|b| {
            let kind = ident(b.name.to_pascal_case());
            let state = ident(b.name.to_shouty_snake_case());
            quote! {
                BlockKind::#kind => BlockState::#state,
            }
        })
        .collect::<TokenStream>();

    let block_kind_variants = blocks
        .iter()
        .map(|b| ident(b.name.to_pascal_case()))
        .collect::<Vec<_>>();

    let block_kind_from_str_arms = blocks
        .iter()
        .map(|b| {
            let name = &b.name;
            let name_ident = ident(name.to_pascal_case());
            quote! {
                #name => Some(BlockKind::#name_ident),
            }
        })
        .collect::<TokenStream>();

    let block_kind_to_str_arms = blocks
        .iter()
        .map(|b| {
            let name = &b.name;
            let name_ident = ident(name.to_pascal_case());
            quote! {
                BlockKind::#name_ident => #name,
            }
        })
        .collect::<TokenStream>();

    let block_kind_props_arms = blocks
        .iter()
        .filter(|&b| !b.properties.is_empty())
        .map(|b| {
            let name = ident(b.name.to_pascal_case());
            let prop_names = b.properties.iter().map(|p| ident(p.name.to_pascal_case()));

            quote! {
                Self::#name => &[#(PropName::#prop_names,)*],
            }
        })
        .collect::<TokenStream>();

    let block_kind_count = blocks.len();

    let prop_names = blocks
        .iter()
        .flat_map(|b| b.properties.iter().map(|p| p.name.as_str()))
        .collect::<BTreeSet<_>>();

    let prop_name_variants = prop_names
        .iter()
        .map(|&name| ident(name.to_pascal_case()))
        .collect::<Vec<_>>();

    let prop_name_from_str_arms = prop_names
        .iter()
        .map(|&name| {
            let ident = ident(name.to_pascal_case());
            quote! {
                #name => Some(PropName::#ident),
            }
        })
        .collect::<TokenStream>();

    let prop_name_to_str_arms = prop_names
        .iter()
        .map(|&name| {
            let ident = ident(name.to_pascal_case());
            quote! {
                PropName::#ident => #name,
            }
        })
        .collect::<TokenStream>();

    let prop_name_count = prop_names.len();

    let prop_values = blocks
        .iter()
        .flat_map(|b| b.properties.iter().flat_map(|p| &p.values))
        .map(|s| s.as_str())
        .collect::<BTreeSet<_>>();

    let prop_value_variants = prop_values
        .iter()
        .map(|val| ident(val.to_pascal_case()))
        .collect::<Vec<_>>();

    let prop_value_from_str_arms = prop_values
        .iter()
        .map(|val| {
            let ident = ident(val.to_pascal_case());
            quote! {
                #val => Some(PropValue::#ident),
            }
        })
        .collect::<TokenStream>();

    let prop_value_to_str_arms = prop_values
        .iter()
        .map(|val| {
            let ident = ident(val.to_pascal_case());
            quote! {
                PropValue::#ident => #val,
            }
        })
        .collect::<TokenStream>();

    let prop_value_from_u16_arms = prop_values
        .iter()
        .filter_map(|v| v.parse::<u16>().ok())
        .map(|n| {
            let ident = ident(n.to_string());
            quote! {
                #n => Some(PropValue::#ident),
            }
        })
        .collect::<TokenStream>();

    let prop_value_to_u16_arms = prop_values
        .iter()
        .filter_map(|v| v.parse::<u16>().ok())
        .map(|n| {
            let ident = ident(n.to_string());
            quote! {
                PropValue::#ident => Some(#n),
            }
        })
        .collect::<TokenStream>();

    let prop_value_count = prop_values.len();

    let items = serde_json::from_str::<Vec<Item>>(include_str!("../extracted/items.json"))?;

    let state_to_item_arms = blocks
        .iter()
        .map(|b| {
            let item_id = b.item_id;

            let item = items.iter().find(|i| i.id == item_id).unwrap();

            if item.id == 0 {
                return quote! {};
            }

            let block_ident = ident(b.name.to_shouty_snake_case());
            let item_ident = ident(item.name.to_pascal_case());

            quote! {
                BlockState::#block_ident => Some(Item::#item_ident),
            }
        })
        .collect::<TokenStream>();

    let item_to_state_arms = items
        .iter()
        .map(|i| {
            let item_id = i.id;
            let item_ident = ident(i.name.to_pascal_case());

            let matching_blocks: Vec<&Block> =
                blocks.iter().filter(|b| b.item_id == item_id).collect();

            if matching_blocks.len() == 1 {
                let state = ident(matching_blocks.get(0).unwrap().name.to_shouty_snake_case());

                quote! {
                    Item::#item_ident => Some(BlockStateType::Normal(BlockState::#state)),
                }
            } else if matching_blocks.len() == 2 {
                let normal_state =
                    ident(matching_blocks.get(0).unwrap().name.to_shouty_snake_case());
                let wall_state = ident(matching_blocks.get(1).unwrap().name.to_shouty_snake_case());

                quote! {
                    Item::#item_ident => Some(BlockStateType::Wall(WallBlockState {
                        normal: BlockState::#normal_state,
                        wall: BlockState::#wall_state
                    })),
                }
            } else if item_ident == ident("Cauldron") {
                let empty_state =
                    ident(matching_blocks.get(0).unwrap().name.to_shouty_snake_case());
                let water_state =
                    ident(matching_blocks.get(1).unwrap().name.to_shouty_snake_case());
                let lava_state = ident(matching_blocks.get(2).unwrap().name.to_shouty_snake_case());
                let powder_snow_state =
                    ident(matching_blocks.get(3).unwrap().name.to_shouty_snake_case());

                quote! {
                    Item::#item_ident => Some(BlockStateType::Cauldren(CauldronBlockState {
                        empty: BlockState::#empty_state,
                        water: BlockState::#water_state,
                        lava: BlockState::#lava_state,
                        powder_snow: BlockState::#powder_snow_state,
                    })),
                }
            } else {
                quote! {}
            }
        })
        .collect::<TokenStream>();

    Ok(quote! {
        /// Represents the state of a block. This does not include block entity data such as
        /// the text on a sign, the design on a banner, or the content of a spawner.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
        pub struct BlockState(u16);

        impl BlockState {
            /// Returns the default block state for a given block type.
            pub const fn from_kind(kind: BlockKind) -> Self {
                match kind {
                    #kind_to_state_arms
                }
            }

            /// Constructs a block state from a raw block state ID.
            ///
            /// If the given ID is invalid, `None` is returned.
            pub const fn from_raw(id: u16) -> Option<Self> {
                if id <= #max_state_id {
                    Some(Self(id))
                } else {
                    None
                }
            }

            pub(crate) const fn from_raw_unchecked(id: u16) -> Self {
                debug_assert!(Self::from_raw(id).is_some());
                Self(id)
            }

            /// Returns the [`BlockKind`] of this block state.
            pub const fn to_kind(self) -> BlockKind {
                match self.0 {
                    #state_to_kind_arms
                    _ => unreachable!(),
                }
            }

            /// Converts this block state to its underlying raw block state ID.
            ///
            /// The original block state can be recovered with [`BlockState::from_raw`].
            pub const fn to_raw(self) -> u16 {
                self.0
            }

            /// Returns the maximum block state ID.
            pub const fn max_raw() -> u16 {
                #max_state_id
            }

            /// Gets the value of the property with the given name from this block.
            ///
            /// If this block does not have the property, then `None` is returned.
            pub const fn get(self, name: PropName) -> Option<PropValue> {
                match self.to_kind() {
                    #get_arms
                    _ => None
                }
            }

            /// Sets the value of a property on this block, returning the modified block.
            ///
            /// If this block does not have the given property or the property value is invalid,
            /// then the original block is returned unchanged.
            #[must_use]
            pub const fn set(self, name: PropName, val: PropValue) -> Self {
                match self.to_kind() {
                    #set_arms
                    _ => self,
                }
            }

            /// If this block is `air`, `cave_air` or `void_air`.
            pub const fn is_air(self) -> bool {
                matches!(
                    self.to_kind(),
                    BlockKind::Air | BlockKind::CaveAir | BlockKind::VoidAir
                )
            }

            // TODO: is_solid

            /// If this block is water or lava.
            pub const fn is_liquid(self) -> bool {
                matches!(self.to_kind(), BlockKind::Water | BlockKind::Lava)
            }

            pub const fn is_opaque(self) -> bool {
                match self.0 {
                    #state_to_opaque_arms
                    _ => true,
                }
            }

            const SHAPES: [[f64; 6]; #shape_count] = [
                #(#shapes,)*
            ];

            pub fn collision_shapes(self) -> impl ExactSizeIterator<Item = vek::Aabb<f64>> + FusedIterator + Clone {
                let shape_idxs: &'static [u16] = match self.0 {
                    #state_to_collision_shapes_arms
                    _ => &[],
                };

                shape_idxs.iter().map(|idx| {
                    let [min_x, min_y, min_z, max_x, max_y, max_z] = Self::SHAPES[*idx as usize];
                    vek::Aabb {
                        min: vek::Vec3::new(min_x, min_y, min_z),
                        max: vek::Vec3::new(max_x, max_y, max_z),
                    }
                })
            }

            pub const fn luminance(self) -> u8 {
                match self.0 {
                    #state_to_luminance_arms
                    _ => 0,
                }
            }

            /// Construct a Item from an BlockState
            ///
            /// If the given BlockState doesn't have a corresponding Item, `None` is returned.
            pub const fn to_item(self) -> Option<Item> {
                match self {
                    #state_to_item_arms
                    _ => None
                }
            }

            /// Construct a BlockStateType from an Item
            ///
            /// If the given Item doesn't have a corresponding block, `None` is returned.
            pub const fn from_item(item: Item) -> Option<BlockStateType> {
                match item {
                    #item_to_state_arms
                    _ => None
                }
            }

            #default_block_states
        }

        /// An enum to store the diffrent result from `from_item`
        ///
        ///
        /// `Normal` is just a single BlockState
        ///
        /// `Wall` is to BlockStates one for the normal variant and one for the wall variant
        ///
        /// `Cauldren` is the diffrent cauldren varients put into one
        pub enum BlockStateType {
            Normal(BlockState),
            Wall(WallBlockState),
            Cauldren(CauldronBlockState)
        }

        /// Stores a normal and a wall variant of an BlockState
        pub struct WallBlockState {
            pub normal: BlockState,
            pub wall: BlockState
        }

        /// Stores the diffrent cauldren variants
        pub struct CauldronBlockState {
            pub empty: BlockState,
            pub water: BlockState,
            pub lava: BlockState,
            pub powder_snow: BlockState
        }

        /// An enumeration of all block kinds.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub enum BlockKind {
            #(#block_kind_variants,)*
        }

        impl BlockKind {
            /// Construct a block kind from its snake_case name.
            ///
            /// Returns `None` if the name is invalid.
            pub fn from_str(name: &str) -> Option<BlockKind> {
                match name {
                    #block_kind_from_str_arms
                    _ => None
                }
            }

            /// Get the snake_case name of this block kind.
            pub const fn to_str(self) -> &'static str {
                match self {
                    #block_kind_to_str_arms
                }
            }

            /// Returns the default block state for a given block kind.
            pub const fn to_state(self) -> BlockState {
                BlockState::from_kind(self)
            }

            /// Returns a slice of all properties this block kind has.
            pub const fn props(self) -> &'static [PropName] {
                match self {
                    #block_kind_props_arms
                    _ => &[],
                }
            }

            pub const fn translation_key(self) -> &'static str {
                match self {
                    #kind_to_translation_key_arms
                }
            }

            /// An array of all block kinds.
            pub const ALL: [Self; #block_kind_count] = [#(Self::#block_kind_variants,)*];
        }

        /// The default block kind is `air`.
        impl Default for BlockKind {
            fn default() -> Self {
                Self::Air
            }
        }

        /// Contains all possible block state property names.
        ///
        /// For example, `waterlogged`, `facing`, and `half` are all property names.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub enum PropName {
            #(#prop_name_variants,)*
        }

        impl PropName {
            /// Construct a property name from its snake_case name.
            ///
            /// Returns `None` if the given name is not valid.
            pub fn from_str(name: &str) -> Option<Self> {
                // TODO: match on str in const fn.
                match name {
                    #prop_name_from_str_arms
                    _ => None,
                }
            }

            /// Get the snake_case name of this property name.
            pub const fn to_str(self) -> &'static str {
                match self {
                    #prop_name_to_str_arms
                }
            }

            /// An array of all property names.
            pub const ALL: [Self; #prop_name_count] = [#(Self::#prop_name_variants,)*];
        }

        /// Contains all possible values that a block property might have.
        ///
        /// For example, `upper`, `true`, and `2` are all property values.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub enum PropValue {
            #(#prop_value_variants,)*
        }

        impl PropValue {
            /// Construct a property value from its snake_case name.
            ///
            /// Returns `None` if the given name is not valid.
            pub fn from_str(name: &str) -> Option<Self> {
                match name {
                    #prop_value_from_str_arms
                    _ => None,
                }
            }

            /// Get the snake_case name of this property value.
            pub const fn to_str(self) -> &'static str {
                match self {
                    #prop_value_to_str_arms
                }
            }

            /// Converts a `u16` into a numeric property value.
            /// Returns `None` if the given number does not have a
            /// corresponding property value.
            pub const fn from_u16(n: u16) -> Option<Self> {
                match n {
                    #prop_value_from_u16_arms
                    _ => None,
                }
            }

            /// Converts this property value into a `u16` if it is numeric.
            /// Returns `None` otherwise.
            pub const fn to_u16(self) -> Option<u16> {
                match self {
                    #prop_value_to_u16_arms
                    _ => None,
                }
            }

            /// Converts a `bool` to a `True` or `False` property value.
            pub const fn from_bool(b: bool) -> Self {
                if b {
                    Self::True
                } else {
                    Self::False
                }
            }

            /// Converts a `True` or `False` property value to a `bool`.
            ///
            /// Returns `None` if this property value is not `True` or `False`
            pub const fn to_bool(self) -> Option<bool> {
                match self {
                    Self::True => Some(true),
                    Self::False => Some(false),
                    _ => None,
                }
            }

            /// An array of all property values.
            pub const ALL: [Self; #prop_value_count] = [#(Self::#prop_value_variants,)*];
        }

        impl From<bool> for PropValue {
            fn from(b: bool) -> Self {
                Self::from_bool(b)
            }
        }
    })
}
