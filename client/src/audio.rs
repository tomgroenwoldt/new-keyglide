use std::io::BufReader;

use anyhow::Result;
use strum::Display;

use crate::config::Config;

#[derive(Display)]
pub enum Audio {
    Reconnected,
}

impl Audio {
    /// # Get asset
    ///
    /// Maps an `Audio` variant to bytes of an MP3 file. The file is embedded
    /// during compile time.
    pub fn get_asset(&self) -> Vec<u8> {
        match self {
            Audio::Reconnected => {
                let file = include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/assets/reconnected.mp3"
                ));
                file.to_vec()
            }
        }
    }
}

/// # Play audio
///
/// Plays an MP3 file until the sound ends. The file is defined by
/// the `Audio` variant and the user provided config.
pub fn play_audio(config: &Config, audio: Audio) -> Result<()> {
    // Get the optional user configuration path for an audio file.
    let path = match audio {
        Audio::Reconnected => &config.audio.reconnected,
    };

    // Setup the audio sink.
    let (_stream, handle) = rodio::OutputStream::try_default()?;
    let sink = rodio::Sink::try_new(&handle)?;

    // Play user specified file. If no file was specified, play the default.
    if let Some(path) = path {
        let file = std::fs::File::open(path)?;
        sink.append(rodio::Decoder::new(BufReader::new(file))?);
    } else {
        let file = std::io::Cursor::new(audio.get_asset());
        sink.append(rodio::Decoder::new(BufReader::new(file))?);
    };

    // Wait for the audio to end.
    sink.sleep_until_end();

    Ok(())
}
