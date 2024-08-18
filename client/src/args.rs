use std::time::Duration;

use anyhow::Result;
use clap::Parser;

use crate::config::Config;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// The application tick rate in milliseconds.
    #[arg(short, long, value_parser = parse_duration, default_value = "35")]
    pub tick_rate: Duration,
    /// Path to a TOML configuration file.
    #[arg(short, long, value_parser = parse_config_from_file_path, default_value = "config.toml")]
    pub config: Config,
    #[arg(short, long, default_value = "keyglide.logs")]
    pub log: String,
}

fn parse_duration(arg: &str) -> Result<std::time::Duration, std::num::ParseIntError> {
    let milliseconds = arg.parse()?;
    Ok(std::time::Duration::from_millis(milliseconds))
}

pub fn parse_config_from_file_path(path: &str) -> Result<Config> {
    let config_file = std::fs::read_to_string(path)
        .expect("Configuration file config.toml should be located in root directory.");
    let config: Config = toml::from_str(&config_file)?;

    // Validate the config during `clap` parsing.
    config.validate()?;
    Ok(config)
}
