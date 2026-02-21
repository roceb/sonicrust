use crate::{
    app::{ActiveSection, ActiveTab, App, InputMode, Track},
    theme::ResolvedTheme,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Tabs},
};
use ratatui_image::StatefulImage;

pub fn draw(f: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());
    let theme = app.config.theme.resolve();
    draw_playback_header(f, app, main_chunks[0], &theme);
    draw_tabs(f, app, main_chunks[1], &theme);
    draw_split_content(f, app, main_chunks[2], &theme);
    // draw_track_list(f, "Queue", app, main_chunks[3]);
    draw_player_controls(f, app, main_chunks[3], &theme);
}

fn draw_playback_header(f: &mut Frame, app: &mut App, area: Rect, theme: &ResolvedTheme) {
    // TODO:Add custom styling form the config file
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_inactive));
    f.render_widget(block.clone(), area);
    let inner_area = block.inner(area);
    // let inner_area = Rect {
    //     x: area.x + 1,
    //     y: area.y + 1,
    //     width: area.width.saturating_sub(2),
    //     height: area.height.saturating_sub(2),
    // };
    if let Some(track) = &app.current_track.clone() {
        let header_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), //Track info
                Constraint::Length(1), //Progress bar
            ])
            .split(inner_area);
        let player_info_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(8), //Cover art area
                Constraint::Min(0),    //Track info
            ])
            .split(header_chunks[0]);
        draw_cover_art(f, app, track, player_info_chunks[0]);
        draw_track_info(f, app, track, player_info_chunks[1], theme);
        draw_progress_bar(f, app, track, header_chunks[1], theme);
    } else {
        let placeholder = Paragraph::new(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("♪ ", Style::default().fg(theme.muted_color)),
                Span::styled(
                    "No track playing",
                    Style::default()
                        .fg(theme.muted_color)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]),
            Line::from(vec![Span::styled(
                "Select a track and press Enter to play",
                Style::default().fg(theme.muted_color),
            )]),
        ])
        .alignment(Alignment::Center);
        f.render_widget(placeholder, inner_area);
    }
}

fn draw_cover_art(f: &mut Frame, app: &mut App, track: &Track, area: Rect) {
    // dont render if it is too small
    if area.width < 2 || area.height < 2 {
        return;
    }
    let has_valid_cover = track
        .cover_art
        .as_ref()
        .map(|url| !url.is_empty())
        .unwrap_or(false);
    if has_valid_cover && let Some(ref mut protocol) = app.cover_art_protocol {
        let image_widget = StatefulImage::default();
        f.render_stateful_widget(image_widget, area, protocol);
        return;
    }
    draw_cover_placeholder(f, area);
}
fn draw_cover_placeholder(f: &mut Frame, area: Rect) {
    let placeholder = Paragraph::new("♪")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(placeholder, area);
}
fn draw_track_info(f: &mut Frame, app: &App, track: &Track, area: Rect, theme: &ResolvedTheme) {
    let bold_mod = if theme.bold {
        Modifier::BOLD
    } else {
        Modifier::empty()
    };
    let status_icon = if app.is_playing { "▶" } else { "⏸" };
    let status_color = if app.is_playing {
        theme.playing_color
    } else {
        Color::LightYellow
    };
    let repeat_indicator = if app.on_repeat {
        Span::styled("repeat: on", Style::default().fg(theme.accent))
    } else {
        Span::styled("repeat: off", Style::default().fg(theme.muted_color))
    };
    let shuffle_indicator = if app._on_shuffle {
        Span::styled("shuffle: on", Style::default().fg(theme.accent))
    } else {
        Span::styled("shuffle: off", Style::default().fg(theme.muted_color))
    };
    let info_lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{} ", status_icon),
                Style::default().fg(status_color).add_modifier(bold_mod),
            ),
            Span::styled(
                &track.title,
                Style::default().fg(theme.fg).add_modifier(bold_mod),
            ),
            Span::styled(" — ", Style::default().fg(theme.muted_color)),
            Span::styled(&track.artist, Style::default().fg(theme.artist_color)),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default()), // Indent to align with title
            // Span::styled("💿 ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                &track.album,
                Style::default()
                    .fg(theme.album_color)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ", Style::default()), // Indent
            // Span::styled("🔊 ", Style::default().fg(Color::DarkGray)),
            // Span::styled(volume_indicator, Style::default().fg(volume_color)),
            Span::styled(
                format!("Volume {:.0}%  ", app.current_volume * 100.0),
                Style::default().fg(theme.fg),
            ),
            repeat_indicator,
            Span::styled("  ", Style::default()), // Indent
            shuffle_indicator,
        ]),
    ];
    let track_info = Paragraph::new(info_lines);
    f.render_widget(track_info, area);
}
fn draw_progress_bar(f: &mut Frame, app: &App, track: &Track, area: Rect, theme: &ResolvedTheme) {
    let current_pos = if let Ok(state) = app.shared_state.read() {
        state.position.as_secs()
    } else {
        0
    };
    let total_duration = track.duration / 1_000_000;
    let progress_ratio = if total_duration > 0 {
        (current_pos as f64 / total_duration as f64).min(1.0)
    } else {
        0.0
    };

    let current_time = format_duration(current_pos);
    let total_time = format_duration(total_duration);
    let time_display = format!("{}/{}", current_time, total_time);

    let gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(theme.playing_color)
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .ratio(progress_ratio)
        .label(Span::styled(
            time_display,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(gauge, area);
}
fn format_duration(sec: i64) -> String {
    let mins = sec / 60;
    let secs = sec % 60;
    format!("{}:{:02}", mins, secs)
}
fn draw_tabs(f: &mut Frame, app: &App, area: Rect, theme: &ResolvedTheme) {
    let tab_titles: Vec<Line> = vec![
        Line::from("Songs"),
        Line::from("Artist"),
        Line::from("Album"),
        Line::from("Playlist"),
    ];
    let selected_tab_index = match app.active_tab {
        ActiveTab::Songs => 0,
        ActiveTab::Artists => 1,
        ActiveTab::Albums => 2,
        ActiveTab::Search => 3,
        ActiveTab::Playlist => 4,
    };
    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT | Borders::BOTTOM),
        )
        .select(selected_tab_index)
        .style(Style::default().fg(theme.muted_color))
        .highlight_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
                .bg(theme.highlight_bg),
        )
        .divider(Span::raw(" | "));
    f.render_widget(tabs, area);
}
fn draw_split_content(f: &mut Frame, app: &mut App, area: Rect, theme: &ResolvedTheme) {
    let content_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60), // Library
            Constraint::Percentage(40), // Queue
        ])
        .split(area);
    let library_active = app.active_section == ActiveSection::Others;
    draw_content_area_with_border(f, app, content_chunks[0], library_active, theme);
    let queue_active = app.active_section == ActiveSection::Queue;
    draw_queue_with_border(f, app, content_chunks[1], queue_active, theme);
}

fn draw_content_area_with_border(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let border_style = if is_active {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    };
    match app.active_tab {
        ActiveTab::Playlist => draw_playlist_styled(f, app, area, border_style, is_active, theme),
        ActiveTab::Albums => draw_album_list_styled(f, app, area, border_style, is_active, theme),
        ActiveTab::Artists => draw_artist_list_styled(f, app, area, border_style, is_active, theme),
        ActiveTab::Songs => draw_song_list_styled(f, app, area, border_style, is_active, theme),
        ActiveTab::Search => draw_search_tab_styled(f, app, area, border_style, is_active, theme),
    }
}

fn draw_queue_with_border(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let border_style = if is_active {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    };
    let title = if is_active {
        format!("Queue ({}) [ACTIVE]", app.queue.len())
    } else {
        format!("Queue ({})", app.queue.len())
    };
    if app.queue.is_empty() {
        let empty_message =
            Paragraph::new("No tracks in queue\n Select a track and press Enter to add")
                .style(Style::default().fg(theme.muted_color))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_style)
                        .title(title),
                );
        f.render_widget(empty_message, area);
        return;
    }
    let tracks: Vec<ListItem> = app
        .queue
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let is_playing = i == app.playing_index && app.current_track.is_some();
            let is_selected = is_active && i == app.selected_queue_index;
            let playing_indicator = if is_playing {
                if app.is_playing { "▶ " } else { "⏸ " }
            } else {
                " "
            };
            let content = vec![Line::from(vec![
                Span::styled(
                    playing_indicator,
                    Style::default().fg(if is_playing {
                        theme.playing_color
                    } else {
                        theme.muted_color
                    }),
                ),
                Span::styled(
                    format!("{:03}. ", i + 1),
                    Style::default().fg(theme.muted_color),
                ),
                Span::styled(
                    &track.title,
                    Style::default().fg(if is_playing {
                        theme.playing_color
                    } else {
                        theme.fg
                    }),
                ),
                Span::styled(
                    format!(" - {}", track.artist),
                    Style::default().fg(theme.artist_color),
                ),
            ])];
            let style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else if is_playing {
                Style::default()
                    .fg(theme.playing_color)
                    .add_modifier(Modifier::ITALIC)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let queue_list = List::new(tracks)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    if is_active {
        if app.selected_queue_index >= app.queue.len() {
            app.selected_queue_index = app.queue.len().saturating_sub(1);
        }
        app.queue_state.select(Some(app.selected_queue_index));
    }
    f.render_stateful_widget(queue_list, area, &mut app.queue_state);
}

fn draw_song_list_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let title = if is_active {
        format!("Songs ({}) [ACTIVE]", app.tracks.len())
    } else {
        format!("Songs ({})", app.tracks.len())
    };
    let tracks: Vec<ListItem> = app
        .tracks
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let is_selected = is_active && i == app.selected_index;
            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{:03}. {} - ", i + 1, track.artist),
                    Style::default().fg(theme.artist_color),
                ),
                Span::styled(&track.title, Style::default().fg(theme.fg)),
                Span::styled(
                    format!(" ({}) ", track.album),
                    Style::default().fg(theme.muted_color),
                ),
            ])];
            let style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();
    let track_list = List::new(tracks)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg),
        )
        .highlight_symbol(">> ");
    if is_active && !app.tracks.is_empty() {
        if app.selected_index >= app.tracks.len() {
            app.selected_index = app.tracks.len().saturating_sub(1);
        }
        app.list_state.select(Some(app.selected_index));
    }
    f.render_stateful_widget(track_list, area, &mut app.list_state);
}

fn draw_search_tab_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    // Split the search area into search input and results
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input box
            Constraint::Min(0),    // Search results
        ])
        .split(area);

    draw_search_input(f, app, chunks[0], theme);
    draw_search_results_styled(f, app, chunks[1], border_style, is_active, theme);
}
fn draw_search_input(f: &mut Frame, app: &App, area: Rect, theme: &ResolvedTheme) {
    let (border_style, _cursor_style) = if app.input_mode == InputMode::Search {
        (
            Style::default().fg(theme.accent),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::RAPID_BLINK),
        )
    } else {
        (Style::default().fg(theme.border_inactive), Style::default())
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
        .style(Style::default().fg(theme.fg))
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

fn draw_search_results_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let title = if is_active {
        format!("Results ({}) [ACTIVE]", app.search_results.len())
    } else {
        format!("Results ({})", app.search_results.len())
    };
    if app.search_results.is_empty() {
        let message = if app.search_query.is_empty() {
            "Enter a search query to find tracks, albums, or artists"
        } else if app.is_searching {
            "Searching..."
        } else {
            "No results found. Try a different search term."
        };

        let empty_message = Paragraph::new(message)
            .style(Style::default().fg(theme.muted_color))
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
            let is_selected = is_active && i == app.selected_search_index;
            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{}. ", i + 1),
                    Style::default().fg(theme.muted_color),
                ),
                Span::styled(
                    format!("{} - ", track.artist),
                    Style::default().fg(theme.artist_color),
                ),
                Span::styled(&track.title, Style::default().fg(theme.fg)),
                Span::styled(
                    format!(" [{}]", track.album),
                    Style::default().fg(theme.album_color),
                ),
            ])];

            let style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(content).style(style)
        })
        .collect();

    let results_list = List::new(results)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Update search state selection
    if is_active && !app.search_results.is_empty() {
        if app.selected_search_index >= app.search_results.len() {
            app.selected_search_index = app.search_results.len().saturating_sub(1);
        }
        app.search_state.select(Some(app.selected_search_index));
    }

    f.render_stateful_widget(results_list, area, &mut app.search_state);
}
fn draw_playlist_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let title = if is_active {
        format!("Playlist ({}) [ACTIVE]", app.playlists.len())
    } else {
        format!("Playlist ({})", app.playlists.len())
    };
    let playlist: Vec<ListItem> = app
        .playlists
        .iter()
        .enumerate()
        .map(|(i, playlist)| {
            let is_selected = is_active && i == app.selected_playlist_index;
            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{} - ", playlist.name),
                    Style::default().fg(theme.fg),
                ),
                Span::styled(
                    format!(" {}", &playlist.song_count),
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    format!(" {}", &playlist.duration),
                    Style::default().fg(theme.muted_color),
                ),
            ])];
            let style = if is_selected {
                Style::default()
                    .bg(theme.muted_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();
    let playlists = List::new(playlist)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg),
        )
        .highlight_symbol(">> ");
    if is_active && !app.playlists.is_empty() {
        if app.selected_playlist_index >= app.playlists.len() {
            app.selected_playlist_index = app.playlists.len().saturating_sub(1);
        }
        app.playlist_state.select(Some(app.selected_playlist_index));
    }
    f.render_stateful_widget(playlists, area, &mut app.playlist_state);
}
fn draw_album_list_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let title = if is_active {
        format!("Albums ({}) [ACTIVE]", app.albums.len())
    } else {
        format!("Albums ({})", app.albums.len())
    };
    let albums: Vec<ListItem> = app
        .albums
        .iter()
        .enumerate()
        .map(|(i, album)| {
            let is_selected = is_active && i == app.selected_album_index;
            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{} - ", album.artist),
                    Style::default().fg(theme.artist_color),
                ),
                Span::raw(&album.name),
            ])];
            let style = if is_selected {
                Style::default()
                    .bg(theme.muted_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();
    let album_list = List::new(albums)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg),
        )
        .highlight_symbol(">> ");
    if is_active && !app.albums.is_empty() {
        if app.selected_album_index >= app.albums.len() {
            app.selected_album_index = app.albums.len().saturating_sub(1);
        }
        app.album_state.select(Some(app.selected_album_index));
    }
    f.render_stateful_widget(album_list, area, &mut app.album_state);
}
fn draw_artist_list_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let title = if is_active {
        format!("Artists ({}) [ACTIVE]", app.artists.len())
    } else {
        format!("Artists ({})", app.artists.len())
    };
    let artists: Vec<ListItem> = app
        .artists
        .iter()
        .enumerate()
        .map(|(i, artist)| {
            let is_selected = is_active && i == app.selected_artist_index;
            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{} - ", artist.name),
                    Style::default().fg(theme.artist_color),
                ),
                Span::raw(artist.album_count.to_string()),
            ])];
            let style = if is_selected {
                Style::default()
                    .bg(theme.muted_color)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();
    let artist_list = List::new(artists)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg),
        )
        .highlight_symbol(">> ");
    if is_active && !app.artists.is_empty() {
        if app.selected_artist_index >= app.artists.len() {
            app.selected_artist_index = app.artists.len().saturating_sub(1);
        }
        app.artist_state.select(Some(app.selected_artist_index));
    }
    // f.render_widget(track_list, area);
    f.render_stateful_widget(artist_list, area, &mut app.artist_state);
}
fn draw_player_controls(f: &mut Frame, app: &App, area: Rect, theme: &ResolvedTheme) {
    let section_indicator = match app.active_section {
        ActiveSection::Queue => "[Queue]",
        ActiveSection::Others => "[Library]",
    };
    let controls = format!(
        "{} Space=Play/Pause ↑/↓=Navigate Enter=Play Tab=Switch Section 1-4=Tabs q=Quit ←/→=Seek +/-=Volume",
        section_indicator
    );
    let controls_widget = Paragraph::new(controls)
        .style(Style::default().fg(theme.fg))
        .block(Block::default().borders(Borders::ALL).title("Controls"));
    f.render_widget(controls_widget, area);
}
