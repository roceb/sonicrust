use mpris_server::zbus;
use mpris_server::zbus::zvariant::ObjectPath;
use tokio::sync::mpsc;

use crate::app::Track;
use crate::player::{PlayerCommand, SharedPlayerState};
// use mpris_server::{Metadata, PlaybackStatus, Player, Time};

use mpris_server::{
    LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, RootInterface, Time,
    TrackId, Volume,
    zbus::{Result, fdo},
};

pub struct MprisPlayer {
    command_tx: mpsc::Sender<PlayerCommand>,
    pub state: SharedPlayerState,
}
impl MprisPlayer {
    pub fn new(command_tx: mpsc::Sender<PlayerCommand>, state: SharedPlayerState) -> Self {
        Self { command_tx, state }
    }
    fn send_command(&self, cmd: PlayerCommand) {
        if let Err(e) = self.command_tx.try_send(cmd) {
            eprintln!("Failed to send MPRIS command: {}", e);
        }
    }
}
impl RootInterface for MprisPlayer {
    async fn raise(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, _fullscreen: bool) -> Result<()> {
        Err(zbus::Error::from(fdo::Error::NotSupported(
            "Fullscreen Not supported".to_string(),
        )))
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn identity(&self) -> fdo::Result<String> {
        Ok("sonicrust".to_string())
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok("com.github.sonicrust".to_string())
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }
    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![])
    }
}

impl PlayerInterface for MprisPlayer {
    async fn next(&self) -> fdo::Result<()> {
        self.send_command(PlayerCommand::Next);
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        self.send_command(PlayerCommand::Previous);
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.send_command(PlayerCommand::Pause);
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        self.send_command(PlayerCommand::TogglePlayPause);
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        self.send_command(PlayerCommand::Stop);
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        self.send_command(PlayerCommand::Play);
        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        let secs = offset.as_micros() / 1_000_000;
        self.send_command(PlayerCommand::SeekRelative(secs));
        Ok(())
    }

    async fn set_position(&self, _track_id: TrackId, position: Time) -> fdo::Result<()> {
        let secs = position.as_micros() / 1_000_000;
        self.send_command(PlayerCommand::SeekAbsolute(secs as u64));
        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Ok(())
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        let state = self.state.read();
        match state {
            Ok(s) => Ok(s.status),
            Err(_) => Ok(PlaybackStatus::Stopped),
        }
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(LoopStatus::None)
    }

    async fn set_loop_status(&self, _loop_status: LoopStatus) -> Result<()> {
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(PlaybackRate::default())
    }

    async fn set_rate(&self, _rate: PlaybackRate) -> Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_shuffle(&self, _shuffle: bool) -> Result<()> {
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        match self.state.read() {
            Ok(s) => Ok(s.metadata.clone()),
            Err(_) => Ok(Metadata::default()),
        }
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(Volume::default())
    }

    async fn set_volume(&self, volume: Volume) -> Result<()> {
        self.send_command(PlayerCommand::SetVolume(volume));
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        let state = self
            .state
            .read()
            .map_err(|_| fdo::Error::Failed("Lock error".into()))?;
        Ok(state.position)
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(PlaybackRate::default())
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(PlaybackRate::default())
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        match self.state.read() {
            Ok(s) => Ok(s.can_go_next),
            Err(_) => Ok(false),
        }
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        match self.state.read() {
            Ok(s) => Ok(s.can_go_previous),
            Err(e) => {
                eprintln!("Error, {}",e);
                Ok(false)},
        }
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}
/// Helper to convert our internal Track crate to MPRIS Metadata
pub fn track_to_metadata(track: &Track) -> Metadata {
    // The Track ID must be a valid D-Bus ObjectPath
    let track_id = ObjectPath::try_from(format!(
        "/org/mpris/MediaPlayer2/Sonicrust/Track/{}",
        track.id
    ))
    .unwrap();
    let art_url = match track.cover_art.clone() {
        Some(url) => url,
        None => "".to_string(),
    };
    Metadata::builder()
        .title(track.title.clone())
        .artist(vec![track.artist.clone()])
        .album(track.album.clone())
        .length(Time::from_micros(track.duration))
        // If you have cover art URLs
        .art_url(art_url)
        .album_artist(track.album_artist.clone())
        .trackid(track_id)
        .track_number(track.track_number.unwrap_or_default())
        .use_count(track.play_count.unwrap_or_default())
        .genre(track.genres.clone())
        .build()
}
