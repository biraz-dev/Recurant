//! Write every sound effect to `target/sfx/*.wav` for auditioning:
//! `cargo run -p zrpg --example sfx`

use zrpg::synth;

fn main() -> std::io::Result<()> {
    let dir = std::path::Path::new("target/sfx");
    std::fs::create_dir_all(dir)?;
    for (name, samples) in synth::all() {
        let path = dir.join(format!("{name}.wav"));
        std::fs::write(&path, synth::wav(&samples))?;
        let secs = samples.len() as f32 / synth::RATE as f32;
        println!("{} ({secs:.2} s)", path.display());
    }
    Ok(())
}
