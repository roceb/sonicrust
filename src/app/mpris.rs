use mpris_server::{PlaybackStatus, Property};
use notify_rust::{Hint, Notification};
use anyhow::Result;

use crate::app::Track;

use super::App;
impl App {
    pub async fn sync_mpris(&mut self) {
        let status = if self.is_playing {
            PlaybackStatus::Playing
        } else if self.current_track.is_some() {
            PlaybackStatus::Paused
        } else {
            PlaybackStatus::Stopped
        };

        let can_next = self.queue_tab.index < self.queue_tab.len().saturating_sub(1);
        let can_prev = self.queue_tab.index > 0;
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

    pub async fn notify_now_playing(&mut self, track: &Track) -> Result<()> {
        let mut notif = Notification::new()
            .appname("Sonicrust")
            .summary("Now playing")
            .body(format!("{} - {}", track.title, track.artist).as_str())
            .finalize();
        if let Some(url) = &track.cover_art
            && !url.is_empty()
        {
            let album = self.sanitize_album_name(&track.album);

            match self.fetch_and_cache_image(url, &album).await {
                Ok(path) => {
                    notif.hint(Hint::ImagePath(path));
                }
                Err(e) => {
                    log::debug!("Could not load cover art for notification: {}", e);
                }
            }
        }
        let _ = notif.show();
        Ok(())
    }
    pub async fn update_mpris_position(&mut self) -> Result<()> {
        if self.is_playing {
            let current_pos = self.player.lock().await.get_position();
            if let Ok(mut state) = self.shared_state.write() {
                state.position = current_pos;
            }
        }
        Ok(())
    }
}
