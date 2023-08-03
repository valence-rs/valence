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

pub fn main() -> anyhow::Result<()> {
    rerun_if_changed(["../../extracted/packets.json"]);

    write_generated_file(build()?, "packet_id.rs")?;

    Ok(())
}

pub fn build() -> anyhow::Result<TokenStream> {
    let packets: Vec<Packet> = serde_json::from_str(include_str!("../../extracted/packets.json"))?;

    let mut consts = TokenStream::new();

    for packet in packets {
        let stripped_name = packet.name.strip_suffix("Packet").unwrap_or(&packet.name);

        let name_ident = ident(stripped_name.to_shouty_snake_case());
        let id = packet.id;

        let doc = format!("Side: {}\n\nState: {}", packet.side, packet.state);

        consts.extend([quote! {
            #[doc = #doc]
            pub const #name_ident: i32 = #id;
        }]);
    }

    Ok(consts)
}
