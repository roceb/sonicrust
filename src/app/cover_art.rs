use std::io::Cursor;

use anyhow::Result;
use image::DynamicImage;
use ratatui_image::picker::Picker;

use crate::app::Track;

use super::App;
impl App {
    pub fn sanitize_album_name(&self, name: &str) -> String {
        name.replace(|c: char| !c.is_alphanumeric() && c != '-', "_")
            .to_lowercase()
            .chars()
            .fold(String::new(), |mut acc, c| {
                if c == '_' && acc.ends_with('_') {
                    acc
                } else {
                    acc.push(c);
                    acc
                }
            })
    }
    pub async fn load_cover_art_for_track(&mut self, track: &Track) {
        self.cover_art_protocol = None;
        let album = self.sanitize_album_name(&track.album);

        let url = match &track.cover_art {
            Some(url) if !url.is_empty() => url,
            _ => return,
        };
        let mut cache_path = std::env::temp_dir();
        cache_path.push("sonicrust");
        cache_path.push(format!("cover_{}.jpg", album));
        let img_result = if cache_path.exists() {
            log::debug!("Using cached cover_art for {}", album);
            image::open(&cache_path)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        } else {
            self.fetch_cover_art(url).await.inspect(|img| {
                let _ = std::fs::create_dir_all(cache_path.parent().unwrap());
                let _ = img.save(&cache_path);
            })
        };
        match img_result {
            Ok(img) => match Picker::from_query_stdio() {
                Ok(picker) => {
                    self.cover_art_protocol = Some(picker.new_resize_protocol(img));
                }
                Err(e) => log::debug!("Failed ot create image picker: {}", e),
            },
            Err(e) => eprintln!("failed to load cover art: {}", e),
        }
    }
    pub async fn fetch_and_cache_image(&self, url: &str, track_album: &str) -> Result<String> {
        log::debug!("Currently fetching cover are for album: {}", track_album,);
        let mut path = std::env::temp_dir();
        path.push("sonicrust");
        std::fs::create_dir_all(&path).map_err(|e| anyhow::anyhow!(e))?;
        path.push(format!("cover_{}.jpg", track_album));
        if path.exists() {
            log::debug!("Using cached cover art for {}", track_album,);
            return Ok(path.to_string_lossy().to_string());
        }
        log::debug!("fetching cover art for {}", track_album);
        let img = self
            .fetch_cover_art(url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch cover art: {}", e))?;
        img.save(&path).map_err(|e| anyhow::anyhow!(e))?;

        Ok(path.to_string_lossy().to_string())
    }

    async fn fetch_cover_art(
        &self,
        url: &str,
    ) -> Result<DynamicImage, Box<dyn std::error::Error + Send + Sync>> {
        let url = url.to_string();
        tokio::task::spawn_blocking(move || {
            let res = reqwest::blocking::get(&url)?;
            if !res.status().is_success() {
                return Err(format!("HTTP error: {}", res.status()).into());
            }
            let bytes = res.bytes()?;
            if bytes.is_empty() {
                return Err("Empty response when fetching cover art".into());
            }
            let format = image::guess_format(&bytes).unwrap_or(image::ImageFormat::Jpeg);
            let img = image::load(Cursor::new(bytes), format)?;
            Ok(img)
        })
        .await?
    }
    pub fn _clear_cover_art_cache() -> Result<()> {
        let mut path = std::env::temp_dir();
        path.push("sonicrust");
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }
        Ok(())
    }
}
