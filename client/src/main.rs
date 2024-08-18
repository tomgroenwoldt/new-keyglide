use std::io;

use anyhow::Result;
use args::Args;
use clap::Parser;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};
use tui_logger::set_log_file;

use crate::app::App;

mod app;
mod args;
#[cfg(feature = "audio")]
mod audio;
mod config;
mod constants;
mod schema;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the logger.
    set_log_file("keyglide.logs")?;
    let drain = tui_logger::Drain::new();
    env_logger::Builder::from_default_env()
        .format(move |_, record| {
            drain.log(record);
            Ok(())
        })
        .init();

    // Parse arguments and configuration file.
    let args = Args::parse();

    // Setup the terminal.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create the app and run it.
    let mut app = App::new(args.config, terminal.size()?).await?;
    let res = app.run(&mut terminal, args.tick_rate).await;

    // Restore the terminal after app termination.
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
