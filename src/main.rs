mod app;
mod collector;
mod control;
mod health;
mod hardware;
mod processes;
mod parser;
mod protocol;
mod ui;

use crate::{app::App, ui::ui};
use crossterm::event::Event::Key;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::{error::Error, io, time::Duration};

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Run app
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Check for new stats
        app.on_tick();

        if event::poll(Duration::from_millis(100))? {
            if let Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('h') => app.toggle_help(),
                    KeyCode::Char('v') => app.cycle_view(),
                    KeyCode::Char('s') => app.toggle_process_sort(),
                    KeyCode::Char('r') => app.request_reconnect(),
                    KeyCode::Char('t') => app.cycle_history_window(),
                    KeyCode::Char('c') => app.control.toggle_jetson_clocks(),
                    KeyCode::Char('m') => app.control.cycle_nvpmodel(),
                    KeyCode::Char('f') => app.control.set_fan(80),
                    _ => {}
                }
            }
        }
    }
}
