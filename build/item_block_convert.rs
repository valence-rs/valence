use crate::{block::Block, ident, item::Item};
use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn block_to_item_arms(blocks: &[Block], items: &[Item]) -> TokenStream {
    blocks
        .iter()
        .map(|b| {
            let item_id = b.item_id;

            let item = items.iter().find(|i| i.id == item_id).unwrap();

            if item.id == 0 {
                return quote! {};
            }

            let block_ident = ident(b.name.to_pascal_case());
            let item_ident = ident(item.name.to_pascal_case());

            quote! {
                BlockKind::#block_ident => Some(ItemKind::#item_ident),
            }
        })
        .collect::<TokenStream>()
}

pub(crate) fn item_to_block_arms(blocks: &[Block], items: &[Item]) -> TokenStream {
    items
        .iter()
        .map(|i| {
            let item_id = i.id;
            let item_ident = ident(i.name.to_pascal_case());

            let matching_blocks: Vec<&Block> =
                blocks.iter().filter(|b| b.item_id == item_id).collect();

            if matching_blocks.len() == 1 {
                let kind = ident(matching_blocks.get(0).unwrap().name.to_pascal_case());

                quote! {
                    ItemKind::#item_ident => Some(BlockKindType::Normal(BlockKind::#kind)),
                }
            } else if matching_blocks.len() == 2 {
                let normal_kind = ident(matching_blocks.get(0).unwrap().name.to_pascal_case());
                let wall_kind = ident(matching_blocks.get(1).unwrap().name.to_pascal_case());

                quote! {
                    ItemKind::#item_ident => Some(BlockKindType::Wall(WallBlockKind {
                        normal: BlockKind::#normal_kind,
                        wall: BlockKind::#wall_kind
                    })),
                }
            } else if item_ident == ident("Cauldron") {
                let empty_kind = ident(matching_blocks.get(0).unwrap().name.to_pascal_case());
                let water_kind = ident(matching_blocks.get(1).unwrap().name.to_pascal_case());
                let lava_kind = ident(matching_blocks.get(2).unwrap().name.to_pascal_case());
                let powder_snow_kind = ident(matching_blocks.get(3).unwrap().name.to_pascal_case());

                quote! {
                    ItemKind::#item_ident => Some(BlockKindType::Cauldron(CauldronBlockKind {
                        empty: BlockKind::#empty_kind,
                        water: BlockKind::#water_kind,
                        lava: BlockKind::#lava_kind,
                        powder_snow: BlockKind::#powder_snow_kind,
                    })),
                }
            } else {
                quote! {}
            }
        })
        .collect::<TokenStream>()
}
