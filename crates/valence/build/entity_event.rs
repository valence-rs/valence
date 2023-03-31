use std::collections::BTreeMap;

use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

use crate::ident;

#[derive(Deserialize, Clone, Debug)]
struct EntityEvents {
    entity_status: BTreeMap<String, u8>,
    entity_animation: BTreeMap<String, u8>,
}

pub fn build() -> anyhow::Result<TokenStream> {
    let entity_events: EntityEvents =
        serde_json::from_str(include_str!("../../../extracted/misc.json"))?;

    let mut statuses: Vec<_> = entity_events.entity_status.into_iter().collect();
    statuses.sort_by_key(|(_, id)| *id);

    let mut animations: Vec<_> = entity_events.entity_animation.into_iter().collect();
    animations.sort_by_key(|(_, id)| *id);

    let entity_status_variants: Vec<_> = statuses
        .iter()
        .map(|(name, code)| {
            let name = ident(name.to_pascal_case());
            let code = *code as isize;

            quote! {
                #name = #code,
            }
        })
        .collect();

    let entity_animation_variants: Vec<_> = animations
        .iter()
        .map(|(name, code)| {
            let name = ident(name.to_pascal_case());
            let code = *code as isize;

            quote! {
                #name = #code,
            }
        })
        .collect();

    Ok(quote! {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum EntityStatus {
            #(#entity_status_variants)*
        }

        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum EntityAnimation {
            #(#entity_animation_variants)*
        }
    })
}
