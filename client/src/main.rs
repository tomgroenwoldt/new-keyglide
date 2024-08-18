use std::{
    io,
    panic::{set_hook, take_hook},
};

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
    // Make sure to restore the terminal state on app crashes.
    init_panic_hook();

    // Parse arguments and configuration file.
    let args = Args::parse();

    // Initialize the logger.
    set_log_file(&args.log)?;
    let drain = tui_logger::Drain::new();
    env_logger::Builder::from_default_env()
        .format(move |_, record| {
            drain.log(record);
            Ok(())
        })
        .init();

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

pub fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

pub fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        // intentionally ignore errors here since we're already in a panic
        let _ = restore_terminal();
        original_hook(panic_info);
    }));
}
