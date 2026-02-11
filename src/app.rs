use crate::{
    config::{Config, SearchMode},
    mpris_handler::{MprisPlayer, track_to_metadata},
    player::{Player, PlayerCommand, PlayerState, SharedPlayerState},
    search::SearchEngine,
    subsonic::SubsonicClient,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use futures::future;
use mpris_server::{Metadata, PlaybackStatus, Property, Server, Time};
use ratatui::widgets::ListState;
use std::{
    rc::Rc,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::{
    sync::{Mutex, mpsc},
    time::interval,
};

pub struct App {
    pub config: Config,
    pub subsonic_client: Arc<SubsonicClient>,
    pub player: Rc<Mutex<Player>>,
    pub queue: Vec<Track>,
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub selected_queue_index: usize,
    pub selected_index: usize,
    pub selected_artist_index: usize,
    pub selected_album_index: usize,
    pub is_playing: bool,
    pub current_track: Option<Track>,
    pub current_volume: f64,
    pub playing_index: usize,
    pub mpris: Server<MprisPlayer>,
    pub shared_state: SharedPlayerState,
    pub command_receiver: mpsc::Receiver<PlayerCommand>,
    pub metadata: Metadata,
    // State manager
    pub queue_state: ListState,
    pub list_state: ListState,
    pub artist_state: ListState,
    pub album_state: ListState,
    pub search_state: ListState,
    pub active_tab: ActiveTab,
    // Search fields
    pub input_mode: InputMode,
    pub search_query: String,
    pub search_results: Vec<Track>,
    pub selected_search_index: usize,
    pub search_engine: SearchEngine,
    pub is_searching: bool,
    // player status
    pub on_repeat: bool,
    // Need to work on the logic to allow shuffling
    pub _on_shuffle: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}
#[derive(Clone, Debug)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album_artist: Option<String>,
    pub album: String,
    pub cover_art: String,
    pub duration: i64,
    pub track_number: Option<i32>,
    // pub genre: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveTab {
    Queue,
    Albums,
    Artists,
    Songs,
    Search,
}
impl App {
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
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
            queue: Vec::new(),
            tracks: Vec::new(),
            artists: Vec::new(),
            albums: Vec::new(),
            metadata: Metadata::default(),
            selected_queue_index: 0,
            selected_index: 0,
            selected_artist_index: 0,
            selected_album_index: 0,
            playing_index: 0,
            is_playing: false,
            current_track: None,
            current_volume: 1.0,
            shared_state,
            queue_state: ListState::default(),
            list_state: ListState::default(),
            artist_state: ListState::default(),
            album_state: ListState::default(),
            search_state: ListState::default(),
            mpris: mprisserver,
            command_receiver: rx,
            active_tab: ActiveTab::Queue,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_results: Vec::new(),
            selected_search_index: 0,
            search_engine,
            is_searching: false,
            on_repeat: false,
            _on_shuffle: true,
        };

        app.refresh_library().await?;
        Ok(app)
    }
    async fn sync_mpris(&mut self) {
        let status = if self.is_playing {
            PlaybackStatus::Playing
        } else if self.current_track.is_some() {
            PlaybackStatus::Paused
        } else {
            PlaybackStatus::Stopped
        };

        let can_next = self.selected_queue_index < self.queue.len().saturating_sub(1);
        let can_prev = self.selected_queue_index > 0;
        let current_pos = self.player.lock().await.get_position();

        if let Ok(mut state) = self.shared_state.write() {
            state.status = status;
            state.metadata = self.metadata.clone();
            state.can_go_next = can_next;
            state.can_go_previous = can_prev;
            state.position = current_pos;
        }

        let _ = self
            .mpris
            .properties_changed([
                Property::PlaybackStatus(status),
                Property::Metadata(self.metadata.clone()),
                Property::CanGoNext(can_next),
                Property::CanGoPrevious(can_prev),
            ])
            .await;
    }
    pub fn enter_search_mode(&mut self) {
        self.input_mode = InputMode::Search;
        self.active_tab = ActiveTab::Search;
        self.search_query.clear();
        self.search_results.clear();
        self.selected_search_index = 0;
        self.search_state.select(None);
    }
    pub fn exit_search_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }
    pub fn search_input(&mut self, c: char) {
        if self.input_mode == InputMode::Search {
            self.search_query.push(c);
            self.is_searching = true
        }
    }
    pub fn search_backspace(&mut self) {
        if self.input_mode == InputMode::Search {
            self.search_query.pop();
            self.is_searching = true;
        }
    }
    pub fn search_clear(&mut self) {
        self.search_query.clear();
        self.search_results.clear();
        self.selected_search_index = 0;
        self.search_state.select(None);
    }
    pub async fn perform_search(&mut self) -> Result<()> {
        if self.search_query.is_empty() {
            self.search_results.clear();
            self.search_state.select(None);
            self.is_searching = false;
            return Ok(());
        }
        match self.config.search.mode {
            SearchMode::Local => {
                self.perform_local_search();
            }
            SearchMode::Remote => {
                self.perform_remote_search().await?;
            }
        }
        self.is_searching = false;
        self.selected_search_index = 0;
        if !self.search_results.is_empty() {
            self.search_state.select(Some(0));
        } else {
            self.search_state.select(None);
        }
        Ok(())
    }

    /// Perform local fuzzy search on loaded tracks
    fn perform_local_search(&mut self) {
        let results = self.search_engine.search(&self.search_query, &self.tracks);
        self.search_results = results.into_iter().map(|r| r.track).collect();
    }

    /// Perform remote search using subsonic api. This is useful for when you have a proxy in
    /// between to search for missing songs
    async fn perform_remote_search(&mut self) -> Result<()> {
        self.search_results = self.subsonic_client.search(&self.search_query).await?;
        Ok(())
    }
    pub async fn play_search_result(&mut self) -> Result<()> {
        if let Some(track) = self.search_results.get(self.selected_search_index).cloned() {
            self.queue = vec![track.clone()];
            self.selected_queue_index = 0;
            self.playing_index = 0;

            let stream_url = self.subsonic_client.get_stream_url(&track.id)?;
            let mut player = self.player.lock().await;
            player.load_url(&stream_url).await?;
            player.play()?;
            self.is_playing = true;
            self.current_track = Some(track.clone());
            self.metadata = track_to_metadata(&track);
            drop(player);
            self.sync_mpris().await;
        }
        Ok(())
    }

    pub fn add_search_result_to_queue(&mut self) {
        if let Some(track) = self.search_results.get(self.selected_search_index).cloned() {
            self.queue.push(track);
        }
    }
    // Add this function to UI
    pub fn _add_all_search_result_to_queue(&mut self) {
        self.queue.extend(self.search_results.clone());
    }

    pub async fn refresh_library(&mut self) -> Result<()> {
        self.tracks = self.subsonic_client.get_all_songs().await?;
        self.artists = self.subsonic_client.get_all_artists().await?;
        self.albums = self.subsonic_client.get_all_albums().await?;
        Ok(())
    }
    pub fn find_selected(&self) -> usize {
        match self.active_tab {
            ActiveTab::Search => {
                if !self.search_results.is_empty() {
                    self.selected_search_index
                } else {
                    0
                }
            }
            ActiveTab::Queue => {
                if !self.queue.is_empty() {
                    self.selected_queue_index
                } else {
                    0
                }
            }
            ActiveTab::Artists => self.selected_queue_index,
            ActiveTab::Songs => {
                if !self.queue.is_empty() {
                    self.selected_queue_index
                } else {
                    0
                }
            }
            ActiveTab::Albums => self.selected_queue_index,
        }
    }
    pub async fn play_selected(&mut self, songindex: usize) -> Result<()> {
        let mut track_to_play: Option<Track> = None;
        match self.active_tab {
            ActiveTab::Queue => {
                track_to_play = self.queue.get(songindex).cloned();
                self.playing_index = songindex;
            }
            ActiveTab::Search => {
                if let Some(track) = self.search_results.get(self.selected_search_index).cloned() {
                    self.queue = self.search_results.clone();
                    self.selected_queue_index = self.selected_search_index;
                    self.playing_index = self.selected_search_index;
                    track_to_play = Some(track);
                }
            }
            ActiveTab::Songs => {
                if let Some(track) = self.tracks.get(self.selected_index).cloned() {
                    self.queue = vec![track.clone()];
                    self.selected_queue_index = 0;
                    track_to_play = Some(track);
                    self.playing_index = self.selected_queue_index;
                }
            }
            ActiveTab::Artists => {
                if let Some(artist) = self.artists.get(self.selected_artist_index) {
                    let artist_albums = self.subsonic_client.get_artist_albums(artist).await?;
                    if !artist_albums.is_empty() {
                        let songs_futures = artist_albums
                            .iter()
                            .map(|album| self.subsonic_client.get_songs_in_album(album));
                        let nested_songs = future::try_join_all(songs_futures).await?;
                        let songs: Vec<Track> = nested_songs.into_iter().flatten().collect();
                        if !songs.is_empty() {
                            self.queue = songs;
                            self.selected_queue_index = 0;
                            track_to_play = self.queue.first().cloned();
                            self.playing_index = self.selected_queue_index;
                        }
                    }
                }
            }
            ActiveTab::Albums => {
                if let Some(album) = self.albums.get(self.selected_album_index) {
                    let songs = self.subsonic_client.get_songs_in_album(album).await?;
                    if !songs.is_empty() {
                        self.queue = songs;
                        self.selected_queue_index = 0;
                        track_to_play = self.queue.first().cloned();
                        self.playing_index = self.selected_queue_index;
                    }
                }
            }
        };
        if let Some(track) = track_to_play {
            let stream_url = self.subsonic_client.get_stream_url(&track.id)?;
            let mut player = self.player.lock().await;
            player.load_url(&stream_url).await?;
            player.play()?;
            self.is_playing = true;
            self.current_track = Some(track.clone());
            self.playing_index = self.selected_queue_index;
            self.metadata = track_to_metadata(&track);
        } else {
            let player = self.player.lock().await;
            player.stop()?;
            self.is_playing = false;
            self.current_track = None;
        }
        self.sync_mpris().await;
        Ok(())
    }

    pub async fn toggle_playback(&mut self) -> Result<()> {
        if self.is_playing {
            let player = self.player.lock().await;
            player.pause()?;
            self.is_playing = false;
        } else if self.current_track.is_some() {
            let player = self.player.lock().await;
            if player.has_track_loaded() {
                player.play()?;
                self.is_playing = true;
            }
        } else if !self.queue.is_empty() {
            self.play_selected(self.playing_index).await?;
            return Ok(());
        }
        self.sync_mpris().await;
        Ok(())
    }

    async fn play_from_queue(&mut self, index: usize) -> Result<()> {
        if let Some(track) = self.queue.get(index).cloned() {
            let stream_url = self.subsonic_client.get_stream_url(&track.id)?;
            let mut player = self.player.lock().await;
            player.load_url(&stream_url).await?;
            player.play()?;
            self.is_playing = true;
            self.current_track = Some(track.clone());
            self.playing_index = index;
            self.metadata = track_to_metadata(&track);
            drop(player);
            self.sync_mpris().await;
        }
        Ok(())
    }
    pub async fn play_next(&mut self) -> Result<()> {
        if !self.queue.is_empty() && self.playing_index < self.queue.len() - 1 {
            self.play_from_queue(self.playing_index + 1).await?;
        } else {
            self.player.lock().await.stop()?;
            self.is_playing = false;
            self.current_track = None;
            self.metadata = Metadata::default();
            self.sync_mpris().await;
        }
        Ok(())
    }

    pub async fn play_previous(&mut self) -> Result<()> {
        if !self.queue.is_empty() && self.playing_index > 0 {
            self.play_from_queue(self.playing_index - 1).await?;
        } else {
            self.player.lock().await.stop()?;
            self.is_playing = false;
            self.metadata = Metadata::default();
            self.sync_mpris().await;
        }
        Ok(())
    }

    pub async fn stop_playback(&mut self) -> Result<()> {
        let player = self.player.lock().await;
        player.stop()?;
        self.is_playing = false;
        self.current_track = None;
        self.metadata = Metadata::default();
        drop(player);
        self.sync_mpris().await;
        Ok(())
    }

    pub async fn set_volume(&mut self, volume: f64) -> Result<()> {
        let clamped = volume.clamp(0.0, 1.0);
        let mut player = self.player.lock().await;
        player.set_volume(clamped as f32)?;

        // Update shared state
        if let Ok(mut state) = self.shared_state.write() {
            state.volume = clamped;
        }

        let _ = self
            .mpris
            .properties_changed([Property::Volume(clamped)])
            .await;

        Ok(())
    }
    pub async fn volume_up(&mut self) -> Result<()> {
        let current = { self.shared_state.read().map(|s| s.volume).unwrap_or(1.0) };
        let new_vol = current + 0.1;
        self.current_volume = new_vol;
        self.set_volume(new_vol).await
    }

    pub async fn volume_down(&mut self) -> Result<()> {
        let current = { self.shared_state.read().map(|s| s.volume).unwrap_or(1.0) };
        let new_vol = current - 0.1;
        self.current_volume = new_vol;
        self.set_volume(new_vol).await
    }
    pub async fn _add_to_queue(&mut self) -> Result<()> {
        todo!()
    }
    pub async fn seek_forward(&mut self) -> Result<()> {
        let player = self.player.lock().await;
        player.seek_relative(5)?;
        Ok(())
    }

    pub async fn seek_backward(&mut self) -> Result<()> {
        let player = self.player.lock().await;
        player.seek_relative(-5)?;
        Ok(())
    }

    pub async fn update(&mut self) -> Result<()> {
        // let mut mpris = self.mpris.lock().await;
        self.check_track_finished().await?;
        self.update_mpris_position().await?;
        while let Ok(cmd) = self.command_receiver.try_recv() {
            println!("Received MPRIS command: {:?}", cmd);
            match cmd {
                PlayerCommand::Play => {
                    if self.current_track.is_some() {
                        let player = self.player.lock().await;
                        player.play()?;
                        self.is_playing = true;
                        drop(player);
                        self.sync_mpris().await;
                    } else if !self.queue.is_empty() {
                        self.play_selected(self.playing_index).await?;
                    }
                }
                PlayerCommand::Pause => {
                    let player = self.player.lock().await;
                    player.pause()?;
                    self.is_playing = false;
                    drop(player);
                    self.sync_mpris().await;
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
        Ok(())
    }
    async fn update_mpris_position(&mut self) -> Result<()> {
        if self.is_playing {
            let current_pos = self.player.lock().await.get_position();
            if let Ok(mut state) = self.shared_state.write() {
                state.position = current_pos;
            }
        }
        Ok(())
    }
    async fn check_track_finished(&mut self) -> Result<()> {
        if !self.is_playing || self.current_track.is_none() {
            return Ok(());
        }
        let is_finished = {
            let player = self.player.lock().await;
            player.is_finished()
        };
        if is_finished {
            self.on_track_finished().await?;
        }
        Ok(())
    }
    async fn on_track_finished(&mut self) -> Result<()> {
        self.subsonic_client
            .scrobble(self.current_track.as_ref().unwrap())
            .await?;
        if self.on_repeat {
            self.play_selected(self.playing_index).await?;
        } else if self.playing_index < self.queue.len().saturating_sub(1) {
            self.play_next().await?;
        } else {
            self.is_playing = false;
            self.current_track = None;
            self.metadata = Metadata::default();
            self.sync_mpris().await;
        }
        Ok(())
    }
    pub fn select_tab(&mut self, tab: ActiveTab) {
        match self.active_tab {
            ActiveTab::Queue => self.queue_state.select(None),
            ActiveTab::Songs => self.list_state.select(None),
            ActiveTab::Artists => self.artist_state.select(None),
            ActiveTab::Albums => self.album_state.select(None),
            ActiveTab::Search => {
                self.search_state.select(None);
                self.input_mode = InputMode::Normal;
            }
        }

        self.active_tab = tab.clone();

        // Initialize new tab state
        match tab {
            ActiveTab::Queue if !self.queue.is_empty() => {
                self.queue_state.select(Some(self.selected_queue_index));
            }
            ActiveTab::Songs if !self.tracks.is_empty() => {
                self.list_state.select(Some(self.selected_index));
            }
            ActiveTab::Artists if !self.artists.is_empty() => {
                self.artist_state.select(Some(self.selected_artist_index));
            }
            ActiveTab::Albums if !self.albums.is_empty() => {
                self.album_state.select(Some(self.selected_album_index));
            }
            ActiveTab::Search if !self.search_results.is_empty() => {
                self.search_state.select(Some(self.selected_search_index));
            }
            _ => {}
        }
    }
    pub fn next_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Queue => {
                self.queue_state.select(None);
                ActiveTab::Songs
            }
            ActiveTab::Songs => {
                self.artist_state.select(None);
                ActiveTab::Artists
            }
            ActiveTab::Artists => {
                self.album_state.select(None);
                ActiveTab::Albums
            }
            ActiveTab::Albums => {
                self.list_state.select(None);
                ActiveTab::Search
            }
            ActiveTab::Search => {
                self.list_state.select(None);
                ActiveTab::Queue
            }
        };
    }
    pub fn previous_tab(&mut self) {
        self.active_tab = match self.active_tab {
            ActiveTab::Artists => {
                self.list_state.select(None);
                ActiveTab::Songs
            }
            ActiveTab::Albums => {
                self.artist_state.select(None);
                ActiveTab::Artists
            }
            ActiveTab::Songs => {
                self.album_state.select(None);
                ActiveTab::Queue
            }
            ActiveTab::Queue => {
                self.queue_state.select(None);
                ActiveTab::Search
            }
            ActiveTab::Search => {
                self.list_state.select(None);
                ActiveTab::Albums
            }
        };
    }
    pub fn next_item_in_tab(&mut self) {
        match self.active_tab {
            ActiveTab::Queue => {
                if !self.queue.is_empty() {
                    let i = if let Some(selected) = self.queue_state.selected() {
                        (selected + 1) % self.queue.len()
                    } else {
                        0
                        // self.queue_state.select(None);
                    };
                    self.selected_queue_index = i;
                    self.queue_state.select(Some(self.selected_queue_index));
                } else {
                    self.queue_state.select(None);
                }
            }
            ActiveTab::Search => {
                if !self.search_results.is_empty() {
                    let i = if let Some(selected) = self.search_state.selected() {
                        (selected + 1) % self.search_results.len()
                    } else {
                        0
                    };
                    self.selected_search_index = i;
                    self.search_state.select(Some(self.selected_search_index));
                } else {
                    self.search_state.select(None);
                }
            }
            ActiveTab::Songs => {
                if !self.tracks.is_empty() {
                    let i = if let Some(selected) = self.list_state.selected() {
                        (selected + 1) % self.tracks.len()
                    } else {
                        0
                    };
                    self.selected_index = i;
                    self.list_state.select(Some(self.selected_index));
                } else {
                    self.list_state.select(None);
                }
            }
            ActiveTab::Artists => {
                if !self.artists.is_empty() {
                    let i = if let Some(selected) = self.artist_state.selected() {
                        (selected + 1) % self.artists.len()
                    } else {
                        0
                    };
                    self.selected_artist_index = i;
                    self.artist_state.select(Some(self.selected_artist_index));
                } else {
                    self.artist_state.select(None);
                }
            }
            ActiveTab::Albums => {
                if !self.albums.is_empty() {
                    let i = if let Some(selected) = self.album_state.selected() {
                        (selected + 1) % self.albums.len()
                    } else {
                        0
                    };
                    self.selected_album_index = i;
                    self.album_state.select(Some(self.selected_album_index));
                } else {
                    self.album_state.select(None);
                }
            }
        }
    }
    pub fn previous_item_in_tab(&mut self) {
        match self.active_tab {
            ActiveTab::Queue => {
                if !self.queue.is_empty() {
                    let i = if let Some(selected) = self.queue_state.selected() {
                        if selected == 0 {
                            self.queue.len() - 1
                        } else {
                            selected - 1
                        }
                    } else {
                        self.queue.len().saturating_sub(1)
                    };
                    self.selected_queue_index = i;
                    self.queue_state.select(Some(self.selected_queue_index));
                } else {
                    self.queue_state.select(None);
                }
            }
            ActiveTab::Search => {
                if !self.search_results.is_empty() {
                    let i = if let Some(selected) = self.search_state.selected() {
                        if selected == 0 {
                            self.search_results.len() - 1
                        } else {
                            selected - 1
                        }
                    } else {
                        self.search_results.len().saturating_sub(1)
                    };
                    self.selected_search_index = i;
                    self.search_state.select(Some(self.selected_search_index));
                } else {
                    self.search_state.select(None);
                }
            }
            ActiveTab::Songs => {
                if !self.tracks.is_empty() {
                    let i = if let Some(selected) = self.list_state.selected() {
                        if selected == 0 {
                            self.tracks.len() - 1
                        } else {
                            selected - 1
                        }
                    } else {
                        self.tracks.len().saturating_sub(1)
                    };
                    self.selected_index = i;
                    self.list_state.select(Some(self.selected_index));
                } else {
                    self.list_state.select(None);
                }
            }
            ActiveTab::Artists => {
                if !self.artists.is_empty() {
                    let i = if let Some(selected) = self.artist_state.selected() {
                        if selected == 0 {
                            self.artists.len() - 1
                        } else {
                            selected - 1
                        }
                    } else {
                        self.artists.len().saturating_sub(1)
                    };
                    self.selected_artist_index = i;
                    self.artist_state.select(Some(self.selected_artist_index));
                } else {
                    self.artist_state.select(None);
                }
            }
            ActiveTab::Albums => {
                if !self.albums.is_empty() {
                    let i = if let Some(selected) = self.album_state.selected() {
                        if selected == 0 {
                            self.albums.len() - 1
                        } else {
                            selected - 1
                        }
                    } else {
                        self.albums.len().saturating_sub(1)
                    };
                    self.selected_album_index = i;
                    self.album_state.select(Some(self.selected_album_index)); // Corrected
                } else {
                    self.album_state.select(None);
                }
            }
        }
    }
    pub async fn handle_search_input(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.exit_search_mode();
            }
            KeyCode::Enter => {
                if !self.search_results.is_empty() {
                    self.perform_search().await?;
                    self.exit_search_mode();
                } else if !self.search_query.is_empty() {
                    self.play_search_result().await?;
                    self.exit_search_mode();
                } else {
                    self.perform_search().await?;
                }
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search_clear();
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.add_search_result_to_queue();
            }
            KeyCode::Char(c) => {
                self.search_input(c);
                //We need a delay here or else every key will perform a search, it can get
                //expensive with big libraries
                interval(Duration::from_millis(100)).tick().await;
                self.perform_search().await?;
            }
            KeyCode::Backspace => {
                self.search_backspace();
                interval(Duration::from_millis(100)).tick().await;
                self.perform_search().await?;
            }
            _ => {}
        }
        Ok(false)
    }
}
