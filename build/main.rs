use proc_macro2::{Ident, Span};

mod block;

pub fn main() -> anyhow::Result<()> {
    for file in ["blocks.json"] {
        println!("cargo:rerun-if-changed=data/{file}");
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
