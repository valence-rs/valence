use std::collections::BTreeMap;

use heck::ToShoutySnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use serde::Deserialize;
use valence_build_utils::{ident, rerun_if_changed, write_generated_file};

#[derive(Deserialize)]
struct Packet {
    name: String,
    side: String,
    state: String,
    id: i32,
}

fn main() -> anyhow::Result<()> {
    rerun_if_changed(["extracted/packets.json"]);

    let packets: Vec<Packet> = serde_json::from_str(include_str!("extracted/packets.json"))?;

    let mut states: BTreeMap<String, TokenStream> = BTreeMap::new();

    for packet in packets {
        let stripped_name = packet.name.strip_suffix("Packet").unwrap_or(&packet.name);

        let name_ident = ident(stripped_name.to_shouty_snake_case());
        let id = packet.id;

        let doc = format!("Side: {}\n\nState: {}", packet.side, packet.state);

        states.entry(packet.state).or_default().extend(quote! {
            #[doc = #doc]
            pub const #name_ident: i32 = #id;
        });
    }

    let out = states
        .into_iter()
        .map(|(state, consts)| {
            let state = ident(state);
            quote! {
                pub mod #state {
                    #consts
                }
            }
        })
        .collect();

    write_generated_file(out, "packet_id.rs")?;

    Ok(())
}
