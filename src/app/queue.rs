use crate::app::{ActiveSection, ActiveTab, Track};
use anyhow::Result;
use futures::future;
use super::App;


impl App {
    pub fn add_search_result_to_queue(&mut self) {
        if let Some(track) = self.search_tab.data.get(self.search_tab.index).cloned() {
            self.queue_tab.data.push(track);
        }
    }
    // Add this function to UI
    pub fn _add_all_search_result_to_queue(&mut self) {
        self.queue_tab.data.extend(self.search_tab.data.clone());
    }

    pub fn find_selected(&self) -> usize {
        match (&self.active_section, &self.active_tab) {
            (ActiveSection::Queue, _) => self.queue_tab.index,
            (ActiveSection::Others, ActiveTab::Songs) => self.tracks_tab.index,
            (ActiveSection::Others, ActiveTab::Artists) => self.artist_tab.index,
            (ActiveSection::Others, ActiveTab::Albums) => self.album_tab.index,
            (ActiveSection::Others, ActiveTab::Playlist) => self.playlist_tab.index,
            (ActiveSection::Others, ActiveTab::Favorites) => self.favorite_tab.index,
            (ActiveSection::Others, ActiveTab::Search) => self.search_tab.index,
        }
    }
    pub async fn _add_to_queue(&mut self) -> Result<()> {
        match (&self.active_section, &self.active_tab) {
            (ActiveSection::Queue, _) => (),
            (ActiveSection::Others, ActiveTab::Songs) => {
                self.queue_tab.data.extend(self.tracks_tab.get().cloned());
            }
            (ActiveSection::Others, ActiveTab::Search) => {
                self.queue_tab.data.extend(self.search_tab.get().cloned());
            }
            (ActiveSection::Others, ActiveTab::Favorites) => {
                self.queue_tab.data.extend(self.favorite_tab.get().cloned());
            }
            (ActiveSection::Others, ActiveTab::Albums) => {
                let album = self.album_tab.get().cloned().unwrap();
                let songs = self.subsonic_client.get_songs_in_album(&album).await?;
                self.queue_tab.data.extend(songs);
            }
            (ActiveSection::Others, ActiveTab::Artists) => {
                let artist = self.artist_tab.get().unwrap();
                let artist_albums = self.subsonic_client.get_artist_albums(artist).await?;
                if !artist_albums.is_empty() {
                    let songs_futures = artist_albums
                        .iter()
                        .map(|album| self.subsonic_client.get_songs_in_album(album));
                    let nested_songs = future::try_join_all(songs_futures).await?;
                    let songs: Vec<Track> = nested_songs.into_iter().flatten().collect();

                    if !songs.is_empty() {
                        self.queue_tab.data.extend(songs);
                    }
                }
            }
            (ActiveSection::Others, ActiveTab::Playlist) => {
                let playlist = self.playlist_tab.get().unwrap();
                let songs = self
                    .subsonic_client
                    .get_songs_from_playlist(playlist)
                    .await?;
                self.queue_tab.data.extend(songs);
            }
        }
        Ok(())
    }
}
