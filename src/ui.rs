use crate::app::{ActiveTab, App, InputMode};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());
    let _theme = app.config.theme.clone();
    draw_header(f, app, main_chunks[0]);
    draw_tabs(f, app, main_chunks[1]);
    draw_content_area(f, app, main_chunks[2]);
    draw_player_controls(f, app, main_chunks[3]);
}

fn draw_header(f: &mut Frame, _app: &App, area: Rect) {
    // TODO:Add custom styling form the config file
    let header = Paragraph::new("SonicRust")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}
fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let tab_titles: Vec<Line> = vec![
        Line::from("Queue"),
        Line::from("Songs"),
        Line::from("Artist"),
        Line::from("Album"),
        Line::from("Search"),
    ];
    let selected_tab_index = match app.active_tab {
        ActiveTab::Queue => 0,
        ActiveTab::Songs => 1,
        ActiveTab::Artists => 2,
        ActiveTab::Albums => 3,
        ActiveTab::Search => 4,
    };
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM))
        .select(selected_tab_index)
        .style(Style::default().fg(Color::LightCyan))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        )
        .divider(Span::raw(" | "));
    f.render_widget(tabs, area);
}
fn draw_content_area(f: &mut Frame, app: &mut App, area: Rect) {
    match app.active_tab {
        ActiveTab::Queue => draw_track_list(f, "Queue", app, area),
        ActiveTab::Albums => draw_album_list(f, app, area),
        ActiveTab::Artists => draw_artist_list(f, app, area),
        ActiveTab::Songs => draw_track_list(f, "Song", app, area),
        ActiveTab::Search => draw_search_tab(f, app, area),
    }
}
fn draw_search_tab(f: &mut Frame, app: &mut App, area: Rect) {
    // Split the search area into search input and results
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input box
            Constraint::Min(0),    // Search results
        ])
        .split(area);

    draw_search_input(f, app, chunks[0]);
    draw_search_results(f, app, chunks[1]);
}
fn draw_search_input(f: &mut Frame, app: &App, area: Rect) {
    let (border_style, _cursor_style) = if app.input_mode == InputMode::Search {
        (
            Style::default().fg(Color::Yellow),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::RAPID_BLINK),
        )
    } else {
        (Style::default().fg(Color::DarkGray), Style::default())
    };

    let search_icon = if app.is_searching { "⏳" } else { "🔍" };

    let input_text = if app.input_mode == InputMode::Search {
        format!("{} {}█", search_icon, app.search_query)
    } else if app.search_query.is_empty() {
        format!("{} Type 's' to start searching...", search_icon)
    } else {
        format!("{} {}", search_icon, app.search_query)
    };

    let mode_indicator = match app.input_mode {
        InputMode::Search => " [SEARCH MODE - Press Esc to exit] ",
        InputMode::Normal => " [Press 's' to search] ",
    };

    let search_input = Paragraph::new(input_text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(Span::styled(
                    format!("Search{}", mode_indicator),
                    border_style,
                )),
        );

    f.render_widget(search_input, area);
}

fn draw_search_results(f: &mut Frame, app: &mut App, area: Rect) {
    if app.search_results.is_empty() {
        let message = if app.search_query.is_empty() {
            "Enter a search query to find tracks, albums, or artists"
        } else if app.is_searching {
            "Searching..."
        } else {
            "No results found. Try a different search term."
        };

        let empty_message = Paragraph::new(message)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title("Results (0)"));
        f.render_widget(empty_message, area);
        return;
    }

    let results: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let content = vec![Line::from(vec![
                Span::styled(format!("{}. ", i + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(
                    format!("{} - ", track.artist),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(&track.title, Style::default().fg(Color::White)),
                Span::styled(
                    format!(" [{}]", track.album),
                    Style::default().fg(Color::Cyan),
                ),
            ])];

            let style = if i == app.selected_search_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let results_title = format!(
        "Results ({}) - Enter=Play | a=Add to Queue | A=Add All | Esc=Exit Search",
        app.search_results.len()
    );

    let results_list = List::new(results)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(results_title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Update search state selection
    if app.search_results.is_empty() {
        app.search_state.select(None);
    } else {
        if app.selected_search_index >= app.search_results.len() {
            app.selected_search_index = app.search_results.len().saturating_sub(1);
        }
        app.search_state.select(Some(app.selected_search_index));
    }

    f.render_stateful_widget(results_list, area, &mut app.search_state);
}
fn draw_album_list(f: &mut Frame, app: &mut App, area: Rect) {
    let albums: Vec<ListItem> = app
        .albums
        .iter()
        .enumerate()
        .map(|(i, album)| {
            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{} - ", album.artist),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(&album.name),
            ])];
            let style = if i == app.selected_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();
    let album_list = List::new(albums)
        .block(Block::default().borders(Borders::ALL).title("Library"))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">> ");
    if app.albums.is_empty() {
        app.album_state.select(None);
    } else {
        if app.selected_album_index >= app.albums.len() {
            app.selected_album_index = app.albums.len().saturating_sub(1);
        }
        app.album_state.select(Some(app.selected_album_index));
    }
    // f.render_widget(track_list, area);
    f.render_stateful_widget(album_list, area, &mut app.album_state);
}
fn draw_artist_list(f: &mut Frame, app: &mut App, area: Rect) {
    let artists: Vec<ListItem> = app
        .artists
        .iter()
        .enumerate()
        .map(|(i, artist)| {
            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{} - ", artist.name),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(artist.album_count.to_string()),
            ])];
            let style = if i == app.selected_artist_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();
    let artist_list = List::new(artists)
        .block(Block::default().borders(Borders::ALL).title("Library"))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">> ");
    if app.artists.is_empty() {
        app.artist_state.select(None);
    } else {
        if app.selected_artist_index >= app.artists.len() {
            app.selected_artist_index = app.artists.len().saturating_sub(1);
        }
        app.artist_state.select(Some(app.selected_artist_index));
    }
    // f.render_widget(track_list, area);
    f.render_stateful_widget(artist_list, area, &mut app.artist_state);
}
fn draw_track_list(f: &mut Frame, type_of_track: &str, app: &mut App, area: Rect) {
    let queue_or_track = match type_of_track {
        "Song" => &app.tracks,
        "Search" => todo!(),
        "Queue" => &app.queue,
        _ => todo!(),
    };
    let mut state_of_list = match type_of_track {
        "Song" => app.list_state,
        "Search" => todo!(),
        "Queue" => app.queue_state,
        _ => todo!(),
    };
    let mut selector = match type_of_track {
        "Song" => app.selected_index,
        "Search" => todo!(),
        "Queue" => app.selected_queue_index,
        _ => todo!(),
    };
    let tracks: Vec<ListItem> = queue_or_track
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{}. {} - ", i + 1, track.artist),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(&track.title),
                Span::styled(
                    format!(" ({})", track.album),
                    Style::default().fg(Color::DarkGray),
                ),
            ])];
            let style = if i == selector {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else if i == app.playing_index {
                Style::default()
                    .add_modifier(Modifier::ITALIC)
                    .fg(Color::LightBlue)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();
    let track_list = List::new(tracks)
        .block(Block::default().borders(Borders::ALL).title(type_of_track))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">> ");

    if queue_or_track.is_empty() {
        state_of_list.select(None);
    } else {
        if selector >= queue_or_track.len() {
            selector = queue_or_track.len().saturating_sub(1);
        }
        state_of_list.select(Some(selector));
    }
    f.render_stateful_widget(track_list, area, &mut state_of_list);
}
fn draw_player_controls(f: &mut Frame, app: &App, area: Rect) {
    let controls = if let Some(track) = &app.current_track {
        let status = if app.is_playing { "▶" } else { "⏸" };
        format!(
            "{} {} - {} 🔉 {:.0}% | Controls: Space=Play/Pause ↑/↓=Navigate Enter=Play q=Quit ←/→=Seek +/-=Volume",
            status,
            track.artist,
            track.title,
            app.current_volume * 100.0
        )
    } else {
        "No track playing | Controls: Space=Play/Pause ↑/↓=Navigate Enter=Play s=search q=Quit"
            .to_string()
    };
    let controls_widget = Paragraph::new(controls)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(controls_widget, area);
}
