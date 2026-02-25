mod app;
mod config;
mod mpris_handler;
mod player;
mod search;
mod subsonic;
mod theme;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use std::{
    io::{self},
    time::Duration,
};
use tokio::time::interval;

use crate::app::InputMode;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let _guard = app::TerminalGuard::new();

    let app = App::new().await?;
    let res = run_app(&mut terminal, app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, &mut app)).unwrap();

        if crossterm::event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if app.input_mode == InputMode::Search {
                app.handle_search_input(key).await?;
            } else if app.input_mode == InputMode::InlineSearch {
                app.handle_inline_search_input(key).await?;
            } else {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char(' ') => app.toggle_playback().await?,
                    KeyCode::Char('k') | KeyCode::Up => app.previous_item_in_tab(),
                    KeyCode::Char('j') | KeyCode::Down => app.next_item_in_tab(),
                    KeyCode::Enter => app.play_selected(app.find_selected()).await?,
                    KeyCode::Left => app.seek_backward().await?,
                    KeyCode::Right => app.seek_forward().await?,
                    KeyCode::Char('r') => app.refresh_library().await?,
                    KeyCode::Char('a') => app._add_to_queue().await?,
                    KeyCode::Char('+') => app.adjust_volume(app::VolumeDirection::Up).await?,
                    KeyCode::Char('-') => app.adjust_volume(app::VolumeDirection::Down).await?,
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.previous_tab(),
                    KeyCode::Char('1') => app.select_tab(app::ActiveTab::Songs),
                    KeyCode::Char('2') => app.select_tab(app::ActiveTab::Artists),
                    KeyCode::Char('3') => app.select_tab(app::ActiveTab::Albums),
                    KeyCode::Char('4') => app.select_tab(app::ActiveTab::Playlist),
                    KeyCode::Char('5') => app.select_tab(app::ActiveTab::Favorites),
                    KeyCode::Char('s') => {
                        app.select_tab(app::ActiveTab::Search);
                        app.enter_search_mode();
                    }
                    KeyCode::Char('/') => {
                        if app.active_tab != app::ActiveTab::Search {
                            app.start_inline_search();
                        }
                    }
                    KeyCode::Char('n') => app.play_next().await?,
                    KeyCode::Char('p') => app.play_previous().await?,
                    _ => {}
                }
            }
        }
        app.update().await?;
        interval(Duration::from_millis(100)).tick().await;
        // tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }
}
