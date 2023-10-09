use valence_build_utils::write_generated_file;

mod block;
mod chunk_view;
mod item;
mod packet_id;
mod sound;
mod status_effect;

pub fn main() -> anyhow::Result<()> {
    write_generated_file(block::build()?, "block.rs")?;
    write_generated_file(status_effect::build()?, "effect.rs")?;
    write_generated_file(item::build()?, "item.rs")?;
    write_generated_file(sound::build()?, "sound.rs")?;
    write_generated_file(packet_id::build()?, "packet_id.rs")?;
    write_generated_file(chunk_view::build(), "chunk_view.rs")?;

    Ok(())
}
