pub mod cover_art;
pub mod input;
pub mod mpris;
pub mod navigation;
pub mod playback;
pub mod queue;
pub mod search;
use crate::{
    config::{Config, ConfigError},
    mpris_handler::{MprisPlayer},
    player::{Player, PlayerCommand, PlayerState, SharedPlayerState},
    search::SearchEngine,
    subsonic::SubsonicClient,
};
use anyhow::Result;
use crossterm::{
    terminal::disable_raw_mode,
};
use mpris_server::{Metadata, PlaybackStatus , Server, Time};
use ratatui::widgets::ListState;
use ratatui_image::{ protocol::StatefulProtocol};
use std::{
    io::{self, Write},
    rc::Rc,
    sync::{Arc, RwLock},
};
use tokio::{
    sync::{Mutex, mpsc},
};

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

pub struct App {
    pub config: Config,
    pub subsonic_client: Arc<SubsonicClient>,
    pub player: Rc<Mutex<Player>>,
    pub is_playing: bool,
    pub current_track: Option<Track>,
    pub current_volume: f64,
    pub playing_index: usize,
    pub mpris: Server<MprisPlayer>,
    pub shared_state: SharedPlayerState,
    pub command_receiver: mpsc::Receiver<PlayerCommand>,
    pub metadata: Metadata,
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
    // player status
    pub on_repeat: bool,
    // Need to work on the logic to allow shuffling
    pub _on_shuffle: bool,
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
        let mpris_interface = MprisPlayer::new(tx.clone(), shared_state.clone());
        let mprisserver = Server::new("sonicrust", mpris_interface)
            .await
            .expect("Unable to build mpris server");
        let search_engine = SearchEngine::new(config.search.fuzzy_threshold, 30);

        let mut app = Self {
            config,
            subsonic_client: subsonic_client.clone(),
            player,
            metadata: Metadata::default(),
            playing_index: 0,
            is_playing: false,
            current_track: None,
            current_volume: 1.0,
            shared_state,
            tracks_tab: TabSelection::new(),
            queue_tab: TabSelection::new(),
            artist_tab: TabSelection::new(),
            album_tab: TabSelection::new(),
            search_tab: TabSelection::new(),
            playlist_tab: TabSelection::new(),
            favorite_tab: TabSelection::new(),
            mpris: mprisserver,
            command_receiver: rx,
            active_tab: ActiveTab::Songs,
            active_section: ActiveSection::Others,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_engine,
            is_searching: false,
            on_repeat: false,
            _on_shuffle: false,
            cover_art_protocol: None,
        };

        app.refresh_library().await?;
        Ok(app)
    }
    pub async fn update(&mut self) -> Result<()> {
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

        self.check_track_finished().await?;
        self.update_mpris_position().await?;
        Ok(())
    }
    pub async fn refresh_library(&mut self) -> Result<()> {
        self.tracks_tab.data = self.subsonic_client.get_all_songs().await?;
        self.artist_tab.data = self.subsonic_client.get_all_artists().await?;
        self.album_tab.data = self.subsonic_client.get_all_albums().await?;
        self.playlist_tab.data = self.subsonic_client.get_playlists().await?;
        self.favorite_tab.data = self.subsonic_client.get_all_favorites().await?;
        Ok(())
    }
}
