use heck::ToShoutySnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use valence_build_utils::{ident, rerun_if_changed};

#[derive(Deserialize)]
struct Packet {
    name: String,
    side: String,
    phase: String,
    id: i32,
}

pub(crate) fn build() -> anyhow::Result<TokenStream> {
    rerun_if_changed(["extracted/packets.json"]);

    let packets: Vec<Packet> = serde_json::from_str(include_str!("../extracted/packets.json"))?;

    let mut consts = TokenStream::new();

    for packet in packets {
        let id = packet.id;
        let sufix = match packet.side.as_str() {
            "serverbound" => "C2S",
            "clientbound" => "S2C",
            _ => unreachable!(),
        };
        
        let name_ident = format_ident!("{}_{}_{}",packet.phase.to_shouty_snake_case(), packet.name.to_shouty_snake_case(), sufix);
        

        let doc = format!("Side: {}\n\nState: {}", packet.side, packet.phase);

        consts.extend([quote! {
            #[doc = #doc]
            pub const #name_ident: i32 = #id;
        }]);
    }

    Ok(consts)
}
