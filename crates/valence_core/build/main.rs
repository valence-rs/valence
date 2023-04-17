use valence_build_utils::{rerun_if_changed, write_generated_file};

mod item;
mod packet_id;
mod sound;
mod translation_key;

pub fn main() -> anyhow::Result<()> {
    rerun_if_changed([
        "../../extracted/items.json",
        "../../extracted/packets.json",
        "../../extracted/sounds.json",
        "../../extracted/translation_keys.json",
    ]);

    write_generated_file(item::build()?, "item.rs")?;
    write_generated_file(sound::build()?, "sound.rs")?;
    write_generated_file(translation_key::build()?, "translation_key.rs")?;
    write_generated_file(packet_id::build()?, "packet_id.rs")?;

    Ok(())
}
