use std::collections::BTreeMap;

use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;

use crate::block::Block;
use crate::ident;
use crate::item::Item;

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
    let blocks_to_block_kind = blocks_to_block_kinds_btree(blocks);

    items
        .iter()
        .map(|i| {
            let item_id = i.id;
            let item_ident = ident(i.name.to_pascal_case());

            let matching_block = blocks.iter().find(|b| b.item_id == item_id);

            match matching_block {
                Some(b) => {
                    let kind = blocks_to_block_kind.get(&b.id).unwrap();
                    let kind_ident = ident(blocks[kind.to_owned() as usize].name.to_pascal_case());

                    quote! {
                    ItemKind::#item_ident => Some(BlockKind::#kind_ident),
                    }
                }
                None => quote! {},
            }
        })
        .collect::<TokenStream>()
}

fn blocks_to_block_kinds_btree(blocks: &[Block]) -> BTreeMap<u16, u16> {
    let mut blocks_to_block_kinds = BTreeMap::new();
    let mut item_ids_used = BTreeMap::new();

    for block in blocks {
        let item_id = block.item_id;
        let block_id = block.id;

        if !item_ids_used.contains_key(&item_id) || item_id == 0 {
            blocks_to_block_kinds.insert(block_id, block_id);
            item_ids_used.insert(item_id, block_id);
        } else {
            let block_state = item_ids_used.get(&item_id).unwrap();
            blocks_to_block_kinds.insert(block_id, block_state.to_owned());
        }
    }
    blocks_to_block_kinds
}
