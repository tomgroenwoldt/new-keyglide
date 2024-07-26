use std::{
    io,
    time::{Duration, Instant},
};

use anyhow::Result;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        self,
        event::{self, DisableMouseCapture, EnableMouseCapture, Event},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    terminal::Terminal,
};

use crate::app::App;

mod app;
mod constants;
mod editor;
mod schema;
mod tab;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    let tick_rate = Duration::from_millis(35);
    run(tick_rate).await?;
    Ok(())
}

pub async fn run(tick_rate: Duration) -> Result<()> {
    // Setup the terminal.
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create the app and run it.
    let app = App::new(terminal.size()?);
    let res = run_app(&mut terminal, app, tick_rate).await;

    // Restore the terminal.
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

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> Result<()> {
    let mut last_tick = Instant::now();
    while !app.exit {
        terminal.draw(|f| ui::draw(f, &app))?;

        // Handle terminal events.
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if crossterm::event::poll(timeout)? {
            let event = event::read()?;

            match event {
                // If the editor is running and focused it takes precedence over
                // anything else.
                Event::Key(key) => {
                    app.on_key(key).await?;
                }
                Event::Resize(cols, rows) => {
                    if let Some(ref mut editor) = app.editor {
                        editor.resize(rows, cols)?;
                    }
                    app.area = terminal.size()?;
                }
                _ => {}
            }
        }

        // Handle app messages sent from other tasks.
        if let Ok(msg) = app.message_rx.try_recv() {
            app.handle_message(msg)?;
        }

        // Handle lobby messages.
        if let Some(ref mut lobby) = app.lobby {
            if let Ok(msg) = lobby.rx.try_recv() {
                lobby.handle_message(msg).await?;
            }
        }

        // Handle application ticks. This is mainly used for handling
        // animations.
        if last_tick.elapsed() >= tick_rate {
            app.on_tick()?;
            last_tick = Instant::now();
        }
    }

    Ok(())
}
