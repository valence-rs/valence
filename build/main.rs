use std::fs;

use anyhow::Context;
use proc_macro2::{Ident, Span};

mod block;

pub fn main() -> anyhow::Result<()> {
    // If any of the files in the data directory are modified, rerun the build
    // script.
    for entry in fs::read_dir("data")? {
        let entry = entry?;
        if entry.metadata()?.is_file() {
            let buf = entry.path();
            let path = buf.to_str().context("bad file name")?;
            println!("cargo:rerun-if-changed={path}");
        }
    }

    block::build()?;

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
