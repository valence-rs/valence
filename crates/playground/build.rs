use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/playground.template.rs");

    let template_path = Path::new("src/playground.template.rs");
    if !template_path.exists() {
        std::fs::copy("src/playground.template.rs", "src/playground.rs").unwrap();
    }
}
