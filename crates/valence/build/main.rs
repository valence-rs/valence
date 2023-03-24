use std::path::Path;
use std::process::Command;
use std::{env, fs};

use anyhow::Context;
use proc_macro2::{Ident, Span};

mod entity;
mod entity_event;

pub fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../../extracted/");

    let generators = [
        (entity::build as fn() -> _, "entity.rs"),
        (entity_event::build, "entity_event.rs"),
    ];

    let out_dir = env::var_os("OUT_DIR").context("can't get OUT_DIR env var")?;

    for (generator, file_name) in generators {
        let path = Path::new(&out_dir).join(file_name);
        let code = generator()?.to_string();
        fs::write(&path, code)?;

        // Format the output for debugging purposes.
        // Doesn't matter if rustfmt is unavailable.
        let _ = Command::new("rustfmt").arg(path).output();
    }

    Ok(())
}

fn ident(s: impl AsRef<str>) -> Ident {
    let s = s.as_ref().trim();

    match s.as_bytes() {
        // TODO: check for the other rust keywords.
        [b'0'..=b'9', ..] | b"type" => Ident::new(&format!("_{s}"), Span::call_site()),
        _ => Ident::new(s, Span::call_site()),
    }
}
