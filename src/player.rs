use anyhow::Result;
use mpris_server::{Metadata, PlaybackStatus, Time};
use rodio::decoder::DecoderBuilder;
use rodio::{OutputStream, OutputStreamBuilder, Sink};
use std::time::Duration;
use std::{
    io::Cursor,
    sync::{Arc, RwLock},
};

pub struct Player {
    // _stream: OutputStream,
    stream_handle: OutputStream,
    sink: Option<Sink>,
    volume: f32,
}

#[derive(Debug)]
pub enum PlayerCommand {
    Play,
    Pause,
    Stop,
    TogglePlayPause,
    Next,
    Previous,
    SetVolume(f64), // Seek(Time)
    SeekRelative(i64),
    SeekAbsolute(u64),
    // TrackFinished,
}
pub struct PlayerState {
    pub status: PlaybackStatus,
    pub metadata: Metadata,
    pub volume: f64,
    pub can_go_next: bool,
    pub can_go_previous: bool,
    pub position: Time,
}

pub type SharedPlayerState = Arc<RwLock<PlayerState>>;

impl Player {
    pub fn new() -> Self {
        let stream_handle =
            OutputStreamBuilder::open_default_stream().expect("open default audio stream");
        Self {
            // _stream: stream,
            stream_handle,
            sink: None,
            volume: 1.0,
        }
    }

    pub async fn load_url(&mut self, url: &str) -> Result<()> {
        // stop current playback
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }

        let resp = reqwest::get(url).await?;
        let bytes = resp.bytes().await?;

        let data_len = bytes.len();
        let cursor = Cursor::new(bytes);
        // let source = Decoder::try_from(cursor)?;
        let source = DecoderBuilder::new()
            .with_data(cursor)
            .with_seekable(true)
            .with_byte_len(data_len as u64)
            .build()?;

        let sink = Sink::connect_new(self.stream_handle.mixer());
        sink.append(source);
        sink.set_volume(self.volume);
        self.sink = Some(sink);

        Ok(())
    }
    pub fn is_finished(&self) -> bool {
        match &self.sink {
            Some(sink) => sink.empty() && !sink.is_paused(),
            None => false,
        }
    }
    pub fn has_track_loaded(&self) -> bool {
        self.sink.is_some()
    }
    pub fn get_position(&self) -> Time {
        if let Some(sink) = &self.sink {
            let duration = sink.get_pos();
            Time::from_micros(duration.as_micros() as i64)
        } else {
            Time::ZERO
        }
    }
    pub fn play(&self) -> Result<()> {
        if let Some(sink) = &self.sink {
            sink.play();
        }
        Ok(())
    }
    pub fn stop(&self) -> Result<()> {
        if let Some(sink) = &self.sink {
            sink.stop();
        }
        Ok(())
    }
    pub fn pause(&self) -> Result<()> {
        if let Some(sink) = &self.sink {
            sink.pause();
        }
        Ok(())
    }
    pub fn seek_relative(&self, delta_sec: i64) -> Result<()> {
        if let Some(sink) = &self.sink {
            let current_pos = sink.get_pos();
            let target = if delta_sec >= 0 {
                current_pos.saturating_add(Duration::from_secs(delta_sec as u64))
            } else {
                let delta = -delta_sec;
                current_pos.saturating_sub(Duration::from_secs(delta as u64))
            };
            if let Err(e) = sink.try_seek(target) {
                eprintln!("Seek failed: {:?}", e);
                return Err(anyhow::anyhow!("Seek failed: {}", e));
            }
        }
        Ok(())
    }
    pub fn seek_absolute(&self, seconds: u64) -> Result<()> {
        if let Some(sink) = &self.sink {
            let _ = sink.try_seek(Duration::from_secs(seconds));
        }
        Ok(())
    }
    pub fn set_volume(&mut self, volume: f32) -> Result<()> {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
        Ok(())
    }
    pub fn _get_volume(&self) -> f32 {
        self.volume
    }
}
