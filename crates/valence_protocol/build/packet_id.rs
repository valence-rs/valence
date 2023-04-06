use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;

use crate::ident;

#[derive(Deserialize)]
struct Packet {
    name: String,
    side: String,
    state: String,
    id: i32,
}

pub fn build() -> anyhow::Result<TokenStream> {
    let packets: Vec<Packet> =
        serde_json::from_str(include_str!("../../../extracted/packets.json"))?;

    let mut consts = TokenStream::new();

    for packet in packets {
        let stripped_name = packet.name.strip_suffix("Packet").unwrap_or(&packet.name);

        let name_ident = ident(stripped_name.to_pascal_case());
        let id = packet.id;

        let doc = format!("Side: {}\nState: {}", packet.side, packet.state);

        consts.extend([quote! {
            #[doc = #doc]
            #[allow(non_upper_case_globals)]
            pub const #name_ident: i32 = #id;
        }]);
    }

    Ok(consts)
}
