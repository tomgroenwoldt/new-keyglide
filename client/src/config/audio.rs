use std::path::Path;

use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Audio {
    pub reconnected: Option<String>,
}

impl Audio {
    pub fn validate(&self) -> Result<()> {
        if let Some(ref reconnected) = self.reconnected {
            let path = Path::new(reconnected);
            if !path.exists() {
                // TODO: Change this error when working on https://github.com/tomgroenwoldt/new-keyglide/issues/25.
                return Err(anyhow!("File {} does not exist...", reconnected));
            }
            let file_extension = path
                .extension()
                .expect("Path should have a file extension.");
            if !file_extension.eq("mp3") {
                // TODO: Change this error when working on https://github.com/tomgroenwoldt/new-keyglide/issues/25.
                return Err(anyhow!("File {} is not MP3...", reconnected));
            }
        }
        Ok(())
    }
}
