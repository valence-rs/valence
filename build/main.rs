use std::path::Path;
use std::process::Command;
use std::{env, fs};

use anyhow::Context;
use proc_macro2::{Ident, Span};

mod block;
mod entity;

pub fn main() -> anyhow::Result<()> {
    for file in ["blocks.json", "entities.json"] {
        println!("cargo:rerun-if-changed=data/{file}");
    }

    block::build()?;
    entity::build()?;

    Ok(())
}

fn ident(s: impl AsRef<str>) -> Ident {
    let s = s.as_ref().trim();
    if s.starts_with(char::is_numeric) {
        Ident::new(&format!("_{s}"), Span::call_site())
    } else {
        Ident::new(s, Span::call_site())
    }
}

fn write_to_out_path(file_name: impl AsRef<str>, content: impl AsRef<str>) -> anyhow::Result<()> {
    let out_dir = env::var_os("OUT_DIR").context("can't get OUT_DIR env var")?;
    let path = Path::new(&out_dir).join(file_name.as_ref());

    fs::write(&path, &content.as_ref())?;

    // Format the output for debugging purposes.
    // Doesn't matter if rustfmt is unavailable.
    let _ = Command::new("rustfmt").arg(path).output();
    Ok(())
}
