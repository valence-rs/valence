use std::collections::BTreeMap;

use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

use crate::ident;

#[derive(Deserialize, Clone, Debug)]
struct EntityData {
    statuses: BTreeMap<String, u8>,
    animations: BTreeMap<String, u8>,
}

pub fn build() -> anyhow::Result<TokenStream> {
    let entity_data: EntityData =
        serde_json::from_str(include_str!("../extracted/entity_data.json"))?;

    let mut statuses: Vec<_> = entity_data.statuses.into_iter().collect();
    statuses.sort_by_key(|(_, id)| *id);

    let mut animations: Vec<_> = entity_data.animations.into_iter().collect();
    animations.sort_by_key(|(_, id)| *id);

    let event_variants = statuses
        .iter()
        .chain(animations.iter())
        .map(|(name, _)| ident(name.to_pascal_case()));

    let status_arms = statuses.iter().map(|(name, code)| {
        let name = ident(name.to_pascal_case());
        quote! {
            Self::#name => StatusOrAnimation::Status(#code),
        }
    });

    let animation_arms = animations.iter().map(|(name, code)| {
        let name = ident(name.to_pascal_case());
        quote! {
            Self::#name => StatusOrAnimation::Animation(#code),
        }
    });

    Ok(quote! {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub enum Event {
            #(#event_variants,)*
        }

        impl Event {
            pub(crate) fn status_or_animation(self) -> StatusOrAnimation {
                match self {
                    #(#status_arms)*
                    #(#animation_arms)*
                }
            }
        }

        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub(crate) enum StatusOrAnimation {
            Status(u8),
            Animation(u8),
        }
    })
}
