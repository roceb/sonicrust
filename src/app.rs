pub mod cover_art;
pub mod input;
pub mod mpris;
pub mod navigation;
pub mod playback;
pub mod queue;
pub mod search;
use crate::{
    config::{Config, ConfigError},
    mpris_handler::MprisPlayer,
    player::{Player, PlayerCommand, PlayerState, SharedPlayerState},
    search::SearchEngine,
    subsonic::SubsonicClient,
};
use anyhow::Result;
use crossterm::terminal::disable_raw_mode;
use mpris_server::{Metadata, PlaybackStatus, Server, Time};
use ratatui::widgets::ListState;
use ratatui_image::protocol::StatefulProtocol;
use std::{
    io::{self, Write},
    rc::Rc,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::{Mutex, mpsc};

pub struct TerminalGuard; // used to make sure terminal goes back to normal
impl TerminalGuard {
    pub fn new() -> Self {
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = crossterm::execute!(
                io::stdout(),
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::event::DisableMouseCapture,
            );
            let _ = io::stdout().flush();
            original_hook(info)
        }));
        Self
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
        );
        let _ = io::stdout().flush();
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("No Track Loaded")]
    NoTrackLoaded,
    #[error("Queue is empty")]
    EmptyQueue,
    #[error("Playback error: {0}")]
    Playback(#[from] anyhow::Error),
}
pub enum VolumeDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatMode {
    None,
    One,
    All,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShuffleMode {
    Off,
    On,
}
pub struct TabSelection<T> {
    pub index: usize,
    pub state: ListState,
    pub data: Vec<T>,
}
impl<T> TabSelection<T> {
    pub fn new() -> Self {
        TabSelection {
            index: 0,
            state: ListState::default(),
            data: Vec::new(),
        }
    }
    pub fn select(&mut self, idx: usize) {
        self.index = idx;
        self.state.select(Some(idx));
    }
    pub fn clear(&mut self) {
        self.state.select(None);
    }
    pub fn current(&mut self) {
        self.state.select(Some(self.index));
    }
    pub fn get(&mut self) -> Option<&T> {
        self.data.get(self.index)
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    InlineSearch, // search in current tab
}
#[derive(Clone, Debug)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album_artist: Option<String>,
    pub album: String,
    pub cover_art: Option<String>,
    pub duration: i64,
    pub track_number: Option<i32>,
    pub play_count: Option<i32>,
    pub genres: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artist: String,
}
#[derive(Clone, Debug)]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub album_count: i32,
}
pub struct Playlists {
    pub id: String,
    pub name: String,
    pub song_count: i32,
    pub duration: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveSection {
    Queue,
    Others,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveTab {
    Playlist,
    Albums,
    Artists,
    Songs,
    Search,
    Favorites,
}

pub enum LibraryMessage {
    Loaded {
        songs: Vec<Track>,
        artists: Vec<Artist>,
        albums: Vec<Album>,
        playlists: Vec<Playlists>,
        favorites: Vec<Track>,
    },
    SongsAppended(Vec<Track>),
    Error(String),
}

pub struct App {
    pub config: Config,
    pub subsonic_client: Arc<SubsonicClient>,
    pub needs_initial_load: bool,
    pub library_rx: Option<mpsc::Receiver<LibraryMessage>>,
    pub player: Rc<Mutex<Player>>,
    pub is_playing: bool,
    pub current_track: Option<Track>,
    pub current_volume: f64,
    pub playing_index: usize,
    pub mpris: Server<MprisPlayer>,
    pub shared_state: SharedPlayerState,
    pub command_receiver: mpsc::Receiver<PlayerCommand>,
    pub metadata: Metadata,
    pub widget_notification: Option<(String, std::time::Instant)>,
    pub w_notification_duration: std::time::Duration,
    pub last_search_keystroke: Option<std::time::Instant>,
    // Shuffle and repeat
    pub on_repeat: RepeatMode,
    pub shuffle_mode: ShuffleMode,
    pub shuffle_order: Vec<usize>,
    pub shuffle_position: usize,
    // TabSelection
    pub queue_tab: TabSelection<Track>,
    pub tracks_tab: TabSelection<Track>,
    pub artist_tab: TabSelection<Artist>,
    pub album_tab: TabSelection<Album>,
    pub playlist_tab: TabSelection<Playlists>,
    pub search_tab: TabSelection<Track>,
    pub favorite_tab: TabSelection<Track>,
    pub active_tab: ActiveTab,
    pub active_section: ActiveSection,
    // Search fields
    pub input_mode: InputMode,
    pub search_query: String,
    pub search_engine: SearchEngine,
    pub is_searching: bool,
    pub cover_art_protocol: Option<StatefulProtocol>,
}

impl App {
    pub async fn new() -> Result<Self> {
        let config = match Config::load() {
            Ok(c) => c,
            Err(ConfigError::NotFound { path }) => {
                eprintln!(
                    "No config found. A default config has been created at: {}\nPlease edit it and restart",
                    path.display()
                );
                std::process::exit(1);
            }
            Err(ConfigError::ParseError { path, reason }) => {
                eprintln!(
                    "Failed to parse config at {}:\n {}\nPlease fix the config and restart.",
                    path.display(),
                    reason
                );
                std::process::exit(1);
            }
            Err(ConfigError::ValidationError(msg)) => {
                eprintln!(
                    "Invalid config value:\n {}\nPlease fix the config and restart.",
                    msg
                );
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("Config error: {}", e);
                std::process::exit(1);
            }
        };
        let subsonic_client = Arc::new(SubsonicClient::new(&config)?);
        let player = Rc::new(Mutex::new(Player::new()));
        let (tx, rx) = mpsc::channel::<PlayerCommand>(32);
        let shared_state = Arc::new(RwLock::new(PlayerState {
            status: PlaybackStatus::Stopped,
            metadata: Metadata::default(),
            volume: 1.0,
            can_go_next: false,
            can_go_previous: false,
            position: Time::ZERO,
        }));
        let mprisserver = {
            let mut result = None;
            for i in 0..10u32 {
                let iface = MprisPlayer::new(tx.clone(), shared_state.clone());
                let name = if i == 0 {
                    "sonicrust".to_string()
                } else {
                    format!("sonicrust.instance{}", i)
                };
                match Server::new(&name, iface).await {
                    Ok(s) => {
                        result = Some(s);
                        break;
                    }
                    Err(e) => {
                        eprintln!("Mpris name '{}' taken, trying next... ({})", name, e);
                    }
                }
            }
            result.ok_or_else(|| {
                anyhow::anyhow!("Failed to register any MPRIS name after 10 attempts")
            })?
        };
        let search_engine = SearchEngine::new(config.search.fuzzy_threshold, 30);

        let app = Self {
            config,
            needs_initial_load: true,
            subsonic_client: subsonic_client.clone(),
            player,
            metadata: Metadata::default(),
            playing_index: 0,
            is_playing: false,
            current_track: None,
            current_volume: 1.0,
            shared_state,
            last_search_keystroke: None,
            widget_notification: None,
            w_notification_duration: std::time::Duration::from_secs(3),
            tracks_tab: TabSelection::new(),
            queue_tab: TabSelection::new(),
            artist_tab: TabSelection::new(),
            album_tab: TabSelection::new(),
            search_tab: TabSelection::new(),
            playlist_tab: TabSelection::new(),
            favorite_tab: TabSelection::new(),
            mpris: mprisserver,
            command_receiver: rx,
            library_rx: None,
            active_tab: ActiveTab::Songs,
            active_section: ActiveSection::Others,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_engine,
            is_searching: false,
            on_repeat: RepeatMode::None,
            shuffle_mode: ShuffleMode::Off,
            shuffle_order: Vec::new(),
            shuffle_position: 0,
            cover_art_protocol: None,
        };

        // app.refresh_library().await?;
        Ok(app)
    }
    pub async fn update(&mut self) -> Result<()> {
        if self.needs_initial_load {
            self.needs_initial_load = false;
            self.start_background_load();
            self.set_notification("Loading Library...");
            // self.refresh_library().await?;
            // self.set_notification("Library loaded");
        }
        if let Some(rx) = &mut self.library_rx {
            match rx.try_recv() {
                Ok(LibraryMessage::Loaded {
                    songs,
                    artists,
                    albums,
                    playlists,
                    favorites,
                }) => {
                    self.tracks_tab.data = songs;
                    self.artist_tab.data = artists;
                    self.album_tab.data = albums;
                    self.playlist_tab.data = playlists;
                    self.favorite_tab.data = favorites;
                    // self.library_rx = None;
                    self.set_notification("Library Loaded");
                }
                Ok(LibraryMessage::SongsAppended(songs)) => {
                    self.tracks_tab.data.extend(songs);
                }
                Ok(LibraryMessage::Error(e)) => {
                    self.set_notification(format!("Load Error: {}", e));
                    self.library_rx = None;
                }
                Err(mpsc::error::TryRecvError::Empty) => {} // This means it is still loading
                Err(_) => {
                    self.library_rx = None;
                }
            }
        }
        while let Ok(cmd) = self.command_receiver.try_recv() {
            match cmd {
                PlayerCommand::Play => {
                    if self.current_track.is_some() {
                        self.is_playing = true;
                        let player = self.player.lock().await;
                        player.play()?;
                        drop(player);
                        self.sync_mpris().await;
                    } else if !self.queue_tab.data.is_empty() {
                        self.play_selected(self.playing_index).await?;
                    }
                }
                PlayerCommand::Pause => {
                    if self.is_playing {
                        self.is_playing = false;
                        let player = self.player.lock().await;
                        if player.has_track_loaded() {
                            player.pause()?;
                        }
                        drop(player);
                        self.sync_mpris().await;
                    }
                }
                PlayerCommand::Stop => {
                    self.stop_playback().await?;
                }
                PlayerCommand::TogglePlayPause => {
                    self.toggle_playback().await?;
                }
                PlayerCommand::SetVolume(v) => {
                    self.set_volume(v).await?;
                }
                PlayerCommand::Next => {
                    self.play_next().await?;
                }
                PlayerCommand::Previous => {
                    self.play_previous().await?;
                }
                // PlayerCommand::TrackFinished => {
                //     self.on_track_finished().await?;
                // }
                PlayerCommand::SeekRelative(secs) => {
                    let player = self.player.lock().await;
                    player.seek_relative(secs)?;
                    let new_pos = player.get_position();
                    if let Ok(mut state) = self.shared_state.write() {
                        state.position = new_pos;
                    }
                }
                PlayerCommand::SeekAbsolute(secs) => {
                    let player = self.player.lock().await;
                    player.seek_absolute(secs)?;
                    let new_pos = player.get_position();
                    if let Ok(mut state) = self.shared_state.write() {
                        state.position = new_pos;
                    }
                }
            }
        }
        if let Some(t) = self.last_search_keystroke
            && t.elapsed() > Duration::from_millis(300)
            && self.is_searching
        {
            self.last_search_keystroke = None;
            self.perform_search().await?;
        }

        self.check_track_finished().await?;
        self.update_mpris_position().await?;
        self.tick_notification();
        Ok(())
    }
    pub fn set_notification(&mut self, msg: impl Into<String>) {
        self.widget_notification = Some((msg.into(), std::time::Instant::now()));
    }
    pub fn tick_notification(&mut self) {
        if let Some((_, created)) = &self.widget_notification
            && created.elapsed() >= self.w_notification_duration
        {
            self.widget_notification = None;
        }
    }
    pub fn start_background_load(&mut self) {
        let (tx, rx) = mpsc::channel(4);
        self.library_rx = Some(rx);
        let client = self.subsonic_client.clone();
        tokio::spawn(async move {
            let (first_page, artists, albums, playlists, favorites) = match tokio::try_join!(
                client.get_album_page(0, 10),
                // client.get_all_songs(),
                client.get_all_artists(),
                client.get_all_albums(),
                client.get_playlists(),
                client.get_all_favorites(),
            ) {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(LibraryMessage::Error(e.to_string())).await;
                    return;
                }
            };
            let first_songs = {
                let futures = first_page.iter().map(|a| client.get_songs_in_album(a));
                futures::future::join_all(futures)
                    .await
                    .into_iter()
                    .flat_map(|r| r.unwrap_or_default())
                    .collect::<Vec<_>>()
            };
            let _ = tx
                .send(LibraryMessage::Loaded {
                    songs: first_songs,
                    artists,
                    albums: albums.clone(),
                    playlists,
                    favorites,
                })
                .await;
            let remaining = albums.iter().skip(10);
            let chunks = remaining.collect::<Vec<_>>();
            for c in chunks.chunks(100) {
                let futures = c.iter().map(|a| client.get_songs_in_album(a));
                let songs = futures::future::join_all(futures)
                    .await
                    .into_iter()
                    .flat_map(|r| r.unwrap_or_default())
                    .collect::<Vec<_>>();
                if tx.send(LibraryMessage::SongsAppended(songs)).await.is_err() {
                    break;
                }
            }
        });
    }
    pub async fn refresh_library(&mut self) -> Result<()> {
        let client = self.subsonic_client.clone();
        self.set_notification("Loading Library...");
        let (songs, artist, albums, playlists, favorites) = tokio::try_join!(
            client.get_all_songs(),
            client.get_all_artists(),
            client.get_all_albums(),
            client.get_playlists(),
            client.get_all_favorites(),
        )?;
        self.tracks_tab.data = songs;
        self.artist_tab.data = artist;
        self.album_tab.data = albums;
        self.playlist_tab.data = playlists;
        self.favorite_tab.data = favorites;

        Ok(())
    }
}
