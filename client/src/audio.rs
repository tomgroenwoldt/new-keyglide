use std::{io::BufReader, path::Path};

use anyhow::Result;

/// # Play audio
///
/// Plays an MP3 file until the sound ends.
pub fn play_audio<P: AsRef<Path>>(path: P) -> Result<()> {
    let file = std::fs::File::open(path)?;
    let (_stream, handle) = rodio::OutputStream::try_default()?;
    let sink = rodio::Sink::try_new(&handle)?;
    sink.append(rodio::Decoder::new(BufReader::new(file))?);
    sink.sleep_until_end();

    Ok(())
}
