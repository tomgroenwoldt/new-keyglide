use std::{io, time::Duration};

use anyhow::Result;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    terminal::Terminal,
};

use crate::app::App;

mod app;
mod constants;
mod schema;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    // Setup the terminal.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create the app and run it.
    let tick_rate = Duration::from_millis(35);
    let mut app = App::new(terminal.size()?).await?;
    let res = app.run(&mut terminal, tick_rate).await;

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
