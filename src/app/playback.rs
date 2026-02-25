use futures::future;
use mpris_server::{Metadata, Property};
use anyhow::Result;

use crate::{app::{ActiveSection, ActiveTab, AppError, Track, VolumeDirection}, mpris_handler::track_to_metadata};

use super::App;
impl App {
    pub async fn toggle_playback(&mut self) -> Result<(), AppError> {
        if self.is_playing {
            let player = self.player.lock().await;
            player.pause()?;
            self.is_playing = false;
        } else if self.current_track.is_none() {
            return Err(AppError::NoTrackLoaded);
        } else if self.current_track.is_some() {
            let player = self.player.lock().await;
            if player.has_track_loaded() {
                player.play()?;
                self.is_playing = true;
                let track = self.current_track.clone().unwrap();
                drop(player);
                self.notify_now_playing(&track).await?;
            }
        } else if !self.queue_tab.data.is_empty() {
            self.play_selected(self.playing_index).await?;
            return Ok(());
        } else if self.queue_tab.data.is_empty() {
            return Err(AppError::EmptyQueue);
        }
        self.sync_mpris().await;
        Ok(())
    }

    async fn start_playback(&mut self, track: Track, queue_index: usize) -> Result<()> {
        let stream_url = self.subsonic_client.get_stream_url(&track.id)?;
        {
            let mut player = self.player.lock().await;
            player.load_url(&stream_url).await?;
            player.play()?;
        }
        self.is_playing = true;
        self.playing_index = queue_index;
        self.current_track = Some(track.clone());
        self.metadata = track_to_metadata(&track);
        self.load_cover_art_for_track(&track).await;
        self.notify_now_playing(&track).await?;
        self.sync_mpris().await;
        self.subsonic_client.scrobble(&track, false).await?;
        Ok(())
    }
    pub async fn play_next(&mut self) -> Result<(), AppError> {
        if self.current_track.is_none() {
            return Err(AppError::NoTrackLoaded);
        }
        if self.queue_tab.data.is_empty() {
            return Err(AppError::EmptyQueue);
        } else if self.playing_index < self.queue_tab.data.len() - 1 {
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

    pub async fn play_previous(&mut self) -> Result<(), AppError> {
        if self.queue_tab.data.is_empty() {
            return Err(AppError::EmptyQueue);
        } else if self.playing_index > 0 {
            self.play_from_queue(self.playing_index - 1).await?;
        } else {
            self.player.lock().await.stop()?;
            self.is_playing = false;
            self.metadata = Metadata::default();
            self.sync_mpris().await;
        }
        Ok(())
    }

    pub async fn stop_playback(&mut self) -> Result<(), AppError> {
        {
            let player = self.player.lock().await;
            player.stop()?;
            self.is_playing = false;
            self.current_track = None;
            self.metadata = Metadata::default();
        }
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
    pub async fn adjust_volume(&mut self, direction: VolumeDirection) -> Result<()> {
        let delta = match direction {
            VolumeDirection::Up => 0.1,
            VolumeDirection::Down => -0.1,
        };
        let current = { self.shared_state.read().map(|s| s.volume).unwrap_or(1.0) };
        self.set_volume(current + delta).await
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
    pub async fn check_track_finished(&mut self) -> Result<()> {
        if !self.is_playing || self.current_track.is_none() {
            return Ok(());
        }
        let is_finished = {
            let player = self.player.lock().await;
            player.is_finished() && player.has_track_loaded()
        };
        if is_finished {
            self.on_track_finished().await?;
        }
        Ok(())
    }
    async fn on_track_finished(&mut self) -> Result<()> {
        self.subsonic_client
            .scrobble(self.current_track.as_ref().unwrap(), true)
            .await?;
        if self.on_repeat {
            self.play_selected(self.playing_index).await?;
        } else if self.playing_index < self.queue_tab.data.len().saturating_sub(1) {
            self.play_next().await?;
        } else {
            self.is_playing = false;
            self.current_track = None;
            self.metadata = Metadata::default();
            self.sync_mpris().await;
        }
        Ok(())
    }
    pub async fn play_search_result(&mut self) -> Result<()> {
        if let Some(track) = self.search_tab.data.get(self.search_tab.index).cloned() {
            self.queue_tab.data = vec![track.clone()];
            self.queue_tab.index = 0;
            self.playing_index = 0;

            self.start_playback(track, self.queue_tab.index).await?;
        }
        Ok(())
    }
    pub async fn play_selected_section(&mut self, songindex: usize) -> Result<()> {
        let mut track_to_play: Option<Track> = None;
        match self.active_section {
            ActiveSection::Queue => {
                track_to_play = self.queue_tab.data.get(songindex).cloned();
                self.playing_index = songindex;
            }
            ActiveSection::Others => (),
        }
        if let Some(track) = track_to_play {
            self.start_playback(track, self.queue_tab.index).await?;
        } else {
            {
                let player = self.player.lock().await;
                player.stop()?;
            }
            self.is_playing = false;
            self.current_track = None;
            self.sync_mpris().await;
        }
        Ok(())
    }
    pub async fn play_selected(&mut self, songindex: usize) -> Result<()> {
        match self.active_section {
            ActiveSection::Queue => {
                return self.play_selected_section(songindex).await;
            }
            ActiveSection::Others => {
                let mut track_to_play: Option<Track> = None;
                match self.active_tab {
                    ActiveTab::Search => {
                        if let Some(track) =
                            self.search_tab.data.get(self.search_tab.index).cloned()
                        {
                            self.queue_tab.data = self.search_tab.data.clone();
                            self.queue_tab.index = self.search_tab.index;
                            self.playing_index = self.search_tab.index;
                            track_to_play = Some(track);
                        }
                    }
                    ActiveTab::Favorites => {
                        if let Some(track) =
                            self.favorite_tab.data.get(self.favorite_tab.index).cloned()
                        {
                            self.queue_tab.data = vec![track.clone()];
                            self.queue_tab.index = 0;
                            track_to_play = Some(track);
                            self.playing_index = self.queue_tab.index;
                        }
                    }
                    ActiveTab::Songs => {
                        if let Some(track) = self.tracks_tab.get().cloned() {
                            self.queue_tab.data = vec![track.clone()];
                            self.queue_tab.index = 0;
                            track_to_play = Some(track);
                            self.playing_index = self.queue_tab.index;
                        }
                    }
                    ActiveTab::Artists => {
                        if let Some(artist) = self.artist_tab.data.get(self.artist_tab.index) {
                            let artist_albums =
                                self.subsonic_client.get_artist_albums(artist).await?;
                            if !artist_albums.is_empty() {
                                let songs_futures = artist_albums
                                    .iter()
                                    .map(|album| self.subsonic_client.get_songs_in_album(album));
                                let nested_songs = future::try_join_all(songs_futures).await?;
                                let songs: Vec<Track> =
                                    nested_songs.into_iter().flatten().collect();
                                if !songs.is_empty() {
                                    self.queue_tab.data = songs;
                                    self.queue_tab.index = 0;
                                    track_to_play = self.queue_tab.data.first().cloned();
                                    self.playing_index = self.queue_tab.index;
                                }
                            }
                        }
                    }
                    ActiveTab::Albums => {
                        if let Some(album) = self.album_tab.data.get(self.album_tab.index) {
                            let songs = self.subsonic_client.get_songs_in_album(album).await?;
                            if !songs.is_empty() {
                                self.queue_tab.data = songs;
                                self.queue_tab.index = 0;
                                track_to_play = self.queue_tab.data.first().cloned();
                                self.playing_index = self.queue_tab.index;
                            }
                        }
                    }
                    ActiveTab::Playlist => {
                        if let Some(playlist) = self.playlist_tab.data.get(self.playlist_tab.index)
                        {
                            let songs = self
                                .subsonic_client
                                .get_songs_from_playlist(playlist)
                                .await?;
                            if !songs.is_empty() {
                                self.queue_tab.data = songs;
                                self.queue_tab.index = 0;
                                track_to_play = self.queue_tab.data.first().cloned();
                                self.playing_index = self.queue_tab.index;
                            }
                        }
                    }
                };
                if let Some(track) = track_to_play {
                    self.start_playback(track.clone(), self.queue_tab.index)
                        .await?;
                } else {
                    let player = self.player.lock().await;
                    player.stop()?;
                    self.is_playing = false;
                    self.current_track = None;
                }
                self.sync_mpris().await;
                Ok(())
            }
        }
    }

    async fn play_from_queue(&mut self, index: usize) -> Result<()> {
        if let Some(track) = self.queue_tab.data.get(index).cloned() {
            self.start_playback(track, index).await?;
        }
        Ok(())
    }
}
