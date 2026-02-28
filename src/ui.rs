use crate::{
    app::{ActiveSection, ActiveTab, App, InputMode, RepeatMode, ShuffleMode, Track},
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

struct StatefulListConfig<'a> {
    items: Vec<ListItem<'a>>,
    area: Rect,
    border_style: Style,
    title: String,
    state: &'a mut ratatui::widgets::ListState,
    selected_index: usize,
    total: usize,
    is_active: bool,
    theme: &'a ResolvedTheme,
}
fn build_list_block<'a>(title: &'a str, border_style: Style) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
}
fn active_title(label: &str, count: usize, is_active: bool) -> String {
    if is_active {
        format!("{} ({}) [ACTIVE]", label, count)
    } else {
        format!("{} ({})", label, count)
    }
}

fn render_stateful_list(f: &mut Frame, config: StatefulListConfig) {
    let list = List::new(config.items)
        .block(build_list_block(&config.title, config.border_style))
        .highlight_style(
            Style::default()
                .bg(config.theme.highlight_bg)
                .fg(config.theme.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");
    if config.is_active && config.total != 0 {
        config
            .state
            .select(Some(config.selected_index.min(config.total - 1)));
    }
    f.render_stateful_widget(list, config.area, config.state);
}

fn active_border_style(is_active: bool, theme: &ResolvedTheme) -> Style {
    if is_active {
        Style::default().fg(theme.border_active)
    } else {
        Style::default().fg(theme.border_inactive)
    }
}
fn build_list_items<'a, T>(
    items: &'a [T],
    selected_index: usize,
    is_active: bool,
    theme: &ResolvedTheme,
    render_item: impl Fn(usize, &'a T) -> Line<'a>,
) -> Vec<ListItem<'a>> {
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = is_active && i == selected_index;
            let style = if is_selected {
                Style::default()
                    .bg(theme.highlight_bg)
                    .fg(theme.highlight_fg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(vec![render_item(i, item)]).style(style)
        })
        .collect()
}
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
    let repeat_indicator = match app.on_repeat {
        RepeatMode::None => Span::styled("repeat: off", Style::default().fg(theme.accent)),
        RepeatMode::One =>
        Span::styled("repeat: one", Style::default().fg(theme.muted_color)),
        RepeatMode::All => Span::styled("repeat: all", Style::default().fg(theme.muted_color))
    };
    let shuffle_indicator = match app.shuffle_mode {
        ShuffleMode::On => Span::styled("shuffle: on", Style::default().fg(theme.accent)),
            ShuffleMode::Off =>Span::styled("shuffle: off", Style::default().fg(theme.muted_color))
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
    let hours = sec / 3600;
    let mins = sec / 60;
    let secs = sec % 60;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, mins, sec)
    } else {
        format!("{}:{:02}", mins, secs)
    }
}
fn draw_tabs(f: &mut Frame, app: &App, area: Rect, theme: &ResolvedTheme) {
    let tab_titles: Vec<Line> = vec![
        Line::from("Songs"),
        Line::from("Artist"),
        Line::from("Album"),
        Line::from("Playlist"),
        Line::from("Favorites"),
    ];
    let selected_tab_index = match app.active_tab {
        ActiveTab::Songs => 0,
        ActiveTab::Artists => 1,
        ActiveTab::Albums => 2,
        ActiveTab::Playlist => 3,
        ActiveTab::Favorites => 4,
        ActiveTab::Search => 5,
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
    let border_style = active_border_style(is_active, theme);
    match app.active_tab {
        ActiveTab::Playlist => draw_playlist_styled(f, app, area, border_style, is_active, theme),
        ActiveTab::Albums => draw_album_list_styled(f, app, area, border_style, is_active, theme),
        ActiveTab::Artists => draw_artist_list_styled(f, app, area, border_style, is_active, theme),
        ActiveTab::Songs => draw_song_list_styled(f, app, area, border_style, is_active, theme),
        ActiveTab::Favorites => {
            draw_favorite_list_styled(f, app, area, border_style, is_active, theme)
        }
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
    let border_style = active_border_style(is_active, theme);
    let title = if is_active {
        format!("Queue ({}) [ACTIVE]", app.queue_tab.len())
    } else {
        format!("Queue ({})", app.queue_tab.len())
    };
    if app.queue_tab.data.is_empty() {
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
    if is_active {
        if app.queue_tab.index >= app.queue_tab.len() {
            app.queue_tab.index = app.queue_tab.len().saturating_sub(1);
        }
        app.queue_tab.current();
    }
    let tracks: Vec<ListItem> = app
        .queue_tab
        .data
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let is_playing = i == app.playing_index && app.current_track.is_some();
            let is_selected = is_active && i == app.queue_tab.index;
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
    f.render_stateful_widget(queue_list, area, &mut app.queue_tab.state);
}

fn draw_favorite_list_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let title = if is_active {
        format!("Favorites ({}) [ACTIVE]", app.favorite_tab.len())
    } else {
        format!("Favorites ({})", app.favorite_tab.len())
    };
    if is_active && !app.favorite_tab.data.is_empty() {
        if app.favorite_tab.index >= app.favorite_tab.len() {
            app.favorite_tab.index = app.favorite_tab.len().saturating_sub(1);
        }
        app.favorite_tab.current();
    }
    let tracks: Vec<ListItem> = app
        .favorite_tab
        .data
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let is_selected = is_active && i == app.favorite_tab.index;
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
    f.render_stateful_widget(track_list, area, &mut app.favorite_tab.state);
}
fn draw_song_list_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let items = build_list_items(
        &app.tracks_tab.data,
        app.tracks_tab.index,
        is_active,
        theme,
        |i, track| {
            Line::from(vec![
                Span::styled(
                    format!("{:03}. {} - ", i + 1, track.artist),
                    theme.artist_color,
                ),
                Span::styled(&track.title, theme.fg),
                Span::styled(format!(" ({}) ", track.album), theme.muted_color),
            ])
        },
    );
    let total = app.tracks_tab.len();
    let title = active_title("Songs", total, is_active);
    render_stateful_list(
        f,
        StatefulListConfig {
            items,
            area,
            border_style,
            title,
            state: &mut app.tracks_tab.state,
            selected_index: app.tracks_tab.index,
            total,
            is_active,
            theme,
        },
    );
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
        InputMode::InlineSearch => "",
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
    let items = build_list_items(
        &app.search_tab.data,
        app.search_tab.index,
        is_active,
        theme,
        |i, track| {
            Line::from(vec![
                Span::styled(
                    format!("{:03}. {} - ", i + 1, track.artist),
                    theme.artist_color,
                ),
                Span::styled(&track.title, theme.fg),
                Span::styled(format!(" ({}) ", track.album), theme.muted_color),
            ])
        },
    );
    let total = app.search_tab.len();
    let title = active_title("Search", total, is_active);
    render_stateful_list(
        f,
        StatefulListConfig {
            items,
            area,
            border_style,
            title,
            state: &mut app.search_tab.state,
            selected_index: app.search_tab.index,
            total,
            is_active,
            theme,
        },
    );
}
fn draw_playlist_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let items = build_list_items(
        &app.playlist_tab.data,
        app.playlist_tab.index,
        is_active,
        theme,
        |i, playlist| {
            Line::from(vec![
                Span::styled(
                    format!("{:03}. {} - ", i + 1, playlist.name),
                    theme.artist_color,
                ),
                Span::styled(format!("{}", &playlist.song_count), theme.fg),
                Span::styled(format!(" ({}) ", playlist.duration), theme.muted_color),
            ])
        },
    );
    let total = app.playlist_tab.len();
    let title = active_title("Playlists", total, is_active);
    render_stateful_list(
        f,
        StatefulListConfig {
            items,
            area,
            border_style,
            title,
            state: &mut app.playlist_tab.state,
            selected_index: app.playlist_tab.index,
            total,
            is_active,
            theme,
        },
    );
}
fn draw_album_list_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let items = build_list_items(
        &app.album_tab.data,
        app.album_tab.index,
        is_active,
        theme,
        |i, album| {
            Line::from(vec![
                Span::styled(
                    format!("{:03}. {} - ", i + 1, album.name),
                    theme.album_color,
                ),
                Span::styled(&album.artist, theme.artist_color),
            ])
        },
    );
    let total = app.album_tab.len();
    let title = active_title("Albums", total, is_active);
    render_stateful_list(
        f,
        StatefulListConfig {
            items,
            area,
            border_style,
            title,
            state: &mut app.album_tab.state,
            selected_index: app.album_tab.index,
            total,
            is_active,
            theme,
        },
    );
}
fn draw_artist_list_styled(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    border_style: Style,
    is_active: bool,
    theme: &ResolvedTheme,
) {
    let items = build_list_items(
        &app.artist_tab.data,
        app.artist_tab.index,
        is_active,
        theme,
        |i, artist| {
            Line::from(vec![
                Span::styled(
                    format!("{:03}. {} - ", i + 1, artist.name),
                    theme.artist_color,
                ),
                Span::styled(format!(" {} ", &artist.album_count), theme.fg),
            ])
        },
    );
    let total = app.artist_tab.len();
    let title = active_title("Artists", total, is_active);
    render_stateful_list(
        f,
        StatefulListConfig {
            items,
            area,
            border_style,
            title,
            state: &mut app.artist_tab.state,
            selected_index: app.artist_tab.index,
            total,
            is_active,
            theme,
        },
    );
}
fn draw_player_controls(f: &mut Frame, app: &App, area: Rect, theme: &ResolvedTheme) {
    let section_indicator = match app.active_section {
        ActiveSection::Queue => "[Queue]",
        ActiveSection::Others => "[Library]",
    };

    // Show notification if active, other show normal controls
    let (controls, border_style, title) = if let Some((msg, _)) = &app.widget_notification {
        (
            format!("* {}", msg),
            Style::default().fg(theme.accent),
            "Notification",
        )
    } else if app.input_mode == InputMode::InlineSearch {
        (
            format!("/ {}█  [Enter/Esc to exit inline search]", app.search_query),
            Style::default().fg(theme.accent),
            "Find",
        )
    } else {
        (
            format!(
                "{} Space=Play/Pause ↑/↓=Navigate Enter=Play Tab=Switch Section 1-4=Tabs q=Quit ←/→=Seek +/-=Volume",
                section_indicator
            ),
            Style::default(),
            "Controls",
        )
    };
    let controls_widget = Paragraph::new(controls)
        .style(
            Style::default().fg(if app.input_mode == InputMode::InlineSearch {
                theme.accent
            } else {
                theme.fg
            }),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title),
        );
    f.render_widget(controls_widget, area);
}
