use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use valence_build_utils::ident;

#[derive(Deserialize, Debug)]
pub struct Sound {
    id: u16,
    name: String,
}

pub fn build() -> anyhow::Result<TokenStream> {
    let sounds =
        serde_json::from_str::<Vec<Sound>>(include_str!("../../../extracted/sounds.json"))?;

    let sound_count = sounds.len();

    let sound_from_raw_id_arms = sounds
        .iter()
        .map(|sound| {
            let id = &sound.id;
            let name = ident(sound.name.to_pascal_case());

            quote! {
                #id => Some(Self::#name),
            }
        })
        .collect::<TokenStream>();

    let sound_to_raw_id_arms = sounds
        .iter()
        .map(|sound| {
            let id = &sound.id;
            let name = ident(sound.name.to_pascal_case());

            quote! {
                Self::#name => #id,
            }
        })
        .collect::<TokenStream>();

    let sound_from_str_arms = sounds
        .iter()
        .map(|sound| {
            let str_name = &sound.name;
            let name = ident(str_name.to_pascal_case());
            quote! {
                #str_name => Some(Self::#name),
            }
        })
        .collect::<TokenStream>();

    let sound_to_str_arms = sounds
        .iter()
        .map(|sound| {
            let str_name = &sound.name;
            let name = ident(str_name.to_pascal_case());
            quote! {
                Self::#name => ident!(#str_name),
            }
        })
        .collect::<TokenStream>();

    let sound_variants = sounds
        .iter()
        .map(|sound| ident(sound.name.to_pascal_case()))
        .collect::<Vec<_>>();

    Ok(quote! {
        /// Represents a sound from the game
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        #[repr(u16)]
        pub enum Sound {
            #(#sound_variants,)*
        }

        impl Sound {
            /// Constructs a sound from a raw item ID.
            ///
            /// If the given ID is invalid, `None` is returned.
            pub const fn from_raw(id: u16) -> Option<Self> {
                match id {
                    #sound_from_raw_id_arms
                    _ => None
                }
            }

            /// Gets the raw sound ID from the sound
            pub const fn to_raw(self) -> u16 {
                match self {
                    #sound_to_raw_id_arms
                }
            }

            /// Construct a sound from its snake_case name.
            ///
            /// Returns `None` if the name is invalid.
            #[allow(clippy::should_implement_trait)]
            pub fn from_str(name: &str) -> Option<Self> {
                match name {
                    #sound_from_str_arms
                    _ => None
                }
            }

            /// Gets the identifier of this sound.
            pub const fn to_ident(self) -> Ident<&'static str> {
                match self {
                    #sound_to_str_arms
                }
            }

            /// An array of all sounds.
            pub const ALL: [Self; #sound_count] = [#(Self::#sound_variants,)*];
        }
    })
}
