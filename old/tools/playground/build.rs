use std::path::Path;

fn main() {
    let current = std::env::current_dir().unwrap();
    println!("current directory: {}", current.display());

    let src = current.join(Path::new("src/playground.template.rs"));
    let dst = current.join(Path::new("src/playground.rs"));

    if dst.exists() {
        println!("{dst:?} already exists, skipping");
        return;
    }

    if !src.exists() {
        println!("{src:?} does not exist, skipping");
        return;
    }

    std::fs::copy(src, dst).unwrap();
}
