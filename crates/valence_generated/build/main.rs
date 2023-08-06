use valence_build_utils::write_generated_file;

mod block;
mod item;
mod translation_key;
mod sound;

pub fn main() -> anyhow::Result<()> {
    write_generated_file(block::build()?, "block.rs")?;
    write_generated_file(translation_key::build()?, "translation_key.rs")?;
    write_generated_file(item::build()?, "item.rs")?;
    write_generated_file(sound::build()?, "sound.rs")?;

    Ok(())
}
