use std::path::Path;
use std::process::Command;
use std::{env, fs};

use anyhow::Context;
use proc_macro2::{Ident as TokenIdent, Span};

mod biome;

pub fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=extracted/");

    let generators = [(biome::build, "biome.rs")];

    let out_dir = env::var_os("OUT_DIR").context("can't get OUT_DIR env var")?;

    for (g, file_name) in generators {
        let path = Path::new(&out_dir).join(file_name);
        let code = g()?.to_string();
        fs::write(&path, code)?;

        // Format the output for debugging purposes.
        // Doesn't matter if rustfmt is unavailable.
        let _ = Command::new("rustfmt").arg(path).output();
    }

    Ok(())
}

fn ident(s: impl AsRef<str>) -> TokenIdent {
    let s = s.as_ref().trim();

    match s.as_bytes() {
        // TODO: check for the other rust keywords.
        [b'0'..=b'9', ..] | b"type" => TokenIdent::new(&format!("_{s}"), Span::call_site()),
        _ => TokenIdent::new(s, Span::call_site()),
    }
}
