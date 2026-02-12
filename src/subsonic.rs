use crate::app::{Album, Artist, Track};
use crate::config::Config;
use anyhow::{Context, Ok, Result};
use serde::Deserialize;
use url::Url;

pub struct SubsonicClient {
    base_url: String,
    username: String,
    password: String,
    secret: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct SubsonicResponseEmpty {
    #[serde(rename = "subsonic-response")]
    subsonic_response: SubsonicResponseInnerEmpty,
}
#[derive(Deserialize)]
struct SubsonicResponse<T> {
    #[serde(rename = "subsonic-response")]
    subsonic_response: SubsonicResponseInner<T>,
}
#[derive(Deserialize, Debug)]
struct SubsonicArtistResponse<T> {
    #[serde(rename = "subsonic-response")]
    subsonic_response: SubsonicArtistResponseInner<T>,
}
#[derive(Deserialize, Debug)]
struct SubsonicArtistResponseInner<T> {
    // #[serde(flatten)]
    artist: T,
}

#[derive(Deserialize)]
struct SubsonicResponseInner<T> {
    #[serde(flatten)]
    data: T,
}

#[derive(Deserialize)]
struct SubsonicResponseInnerEmpty {
    status: String,
    #[serde(rename = "version")]
    _version: String,
    #[serde(rename = "type")]
    _type_resp: String,
    #[serde(rename = "openSubsonic")]
    _open_subsonic: bool,
}

// #[derive(Deserialize)]
// struct ArtistInfo {
//     id: String,
//     name: String,
// }
#[derive(Deserialize)]
struct GetArtistsListResponse {
    // #[serde(rename = "albumList2")]
    artists: ArtistList,
}
#[derive(Deserialize)]
struct ArtistList {
    #[serde(rename = "index")]
    artist_index: Vec<LetterArtist>,
}
#[derive(Deserialize)]
struct LetterArtist {
    artist: Vec<ArtistInfo>,
    // name: String,
}
#[derive(Deserialize, Debug)]
struct ArtistInfo {
    id: String,
    name: String,
    // #[serde(rename = "coverArt")]
    // cover_art: String,
    #[serde(rename = "albumCount")]
    album_count: i32,
}

#[derive(Deserialize)]
struct GetAlbumListResponse {
    #[serde(rename = "albumList2")]
    album_list: AlbumList,
}
#[derive(Deserialize, Debug)]
struct AlbumList {
    album: Vec<AlbumInfo>,
}

#[derive(Deserialize, Debug)]
struct AlbumInfo {
    id: String,
    name: String,
    artist: String,
}
#[derive(Deserialize)]
struct GetAlbumResponse {
    album: AlbumDetail,
}
#[derive(Deserialize)]
struct AlbumDetail {
    song: Vec<Song>,
}
#[derive(Deserialize)]
struct Discography {
    album: Vec<AlbumInfo>,
}

#[derive(Deserialize, Debug)]
struct Song {
    id: String,
    title: String,
    artist: String,
    album: String,
    duration: Option<i64>,
    #[serde(rename = "track")]
    track_number: Option<i32>,
    // genre: Option<String>,
    #[serde(rename = "playCount")]
    play_count: Option<i32>,
    #[serde(rename = "displayAlbumArtist")]
    display_album_artist: Option<String>,
    genres: Vec<Genres>,
}

#[derive(Deserialize, Debug)]
struct Genres {
    name: String,
}

#[derive(Deserialize, Debug)]
struct SearchResult3 {
    #[serde(rename = "searchResult3")]
    search_result: SearchResultInner,
}

#[derive(Deserialize, Debug)]
struct SearchResultInner {
    #[serde(default)]
    song: Vec<Song>,
    #[serde(default)]
    _album: Vec<AlbumInfo>,
    #[serde(default)]
    _artist: Vec<ArtistInfo>,
}

impl SubsonicClient {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            base_url: config.server_url.clone(),
            username: config.username.clone(),
            password: config.password.clone(),
            secret: config.secret.clone(),
            client: reqwest::Client::new(),
        })
    }
    fn get_auth_params(&self) -> Vec<(&str, String)> {
        let salt = &self.secret; // "Secretsaltshaker2000";
        let token = format!("{:x}", md5::compute(format!("{}{}", self.password, salt)));
        vec![
            ("u", self.username.clone()),
            ("t", token),
            ("s", salt.to_string()),
            ("v", "1.16.1".to_string()),
            ("c", "sonicrust".to_string()),
            ("f", "json".to_string()),
        ]
    }
    pub async fn search(&self, search: &str) -> Result<Vec<Track>> {
        if search.is_empty() {
            return Ok(Vec::new());
        }
        let mut url = Url::parse(&format!("{}/rest/search3", self.base_url))?;
        let mut params = self.get_auth_params();
        params.push(("query", search.to_string()));
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, &value);
        }
        let res: SubsonicResponse<SearchResult3> =
            self.client.get(url).send().await?.json().await?;
        let mut tracks = Vec::new();

        for song in res.subsonic_response.data.search_result.song {
            let mut cover_art_url = Url::parse(&format!("{}/rest/getCoverArt", self.base_url))?;
            let mut cover_art_params = self.get_auth_params();
            cover_art_params.push(("id", song.id.clone()));
            for (key, value) in cover_art_params {
                cover_art_url.query_pairs_mut().append_pair(key, &value);
            }
            let duration = song.duration.unwrap_or(0) * 1_000_000;
            tracks.push(Track {
                id: song.id,
                title: song.title,
                artist: song.artist,
                album_artist: song.display_album_artist,
                album: song.album,
                cover_art: Some(cover_art_url.to_string()),
                duration,
                track_number: song.track_number,
                // genre: song.genre,
                play_count: song.play_count,
                genres: song.genres.iter().map(|f| f.name.clone()).collect(),
            });
        }

        Ok(tracks)
    }
    pub async fn get_all_albums(&self) -> Result<Vec<Album>> {
        // TODO: Add paginating to help with big libraries
        let mut url = Url::parse(&format!("{}/rest/getAlbumList2", self.base_url))?;
        let mut params = self.get_auth_params();
        params.push(("type", "alphabeticalByArtist".to_string()));
        params.push(("size", "500".to_string()));
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, &value);
        }
        let mut albums = Vec::new();
        let response: SubsonicResponse<GetAlbumListResponse> =
            self.client.get(url).send().await?.json().await?;
        let albums_response = response.subsonic_response.data.album_list.album;
        for album in albums_response {
            albums.push(Album {
                id: album.id,
                name: album.name,
                artist: album.artist,
            });
        }
        Ok(albums)
    }
    pub async fn get_all_artists(&self) -> Result<Vec<Artist>> {
        let mut artists = Vec::new();
        let mut url = Url::parse(&format!("{}/rest/getArtists", self.base_url))?;
        let params = self.get_auth_params();
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, &value);
        }
        let response: SubsonicResponse<GetArtistsListResponse> =
            self.client.get(url).send().await?.json().await?;
        for letter_artist in response.subsonic_response.data.artists.artist_index {
            for artist in letter_artist.artist {
                artists.push(Artist {
                    id: artist.id,
                    name: artist.name,
                    album_count: artist.album_count,
                });
            }
        }

        Ok(artists)
    }
    pub async fn get_artist_albums(&self, artist: &Artist) -> Result<Vec<Album>> {
        let mut albums: Vec<Album> = Vec::new();
        let mut url = Url::parse(&format!("{}/rest/getArtist", self.base_url))?;
        let mut params = self.get_auth_params();
        params.push(("id", artist.id.clone()));
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, &value);
        }
        tracing::debug!("Requesting artist albums URL: {}", url);
        // TODO: debug why this fails
        let res = self
            .client
            .get(url.clone())
            .send()
            .await
            .with_context(|| format!("Failed to send request to {}", url))?;
        let body_text = res.text().await?;
        tracing::debug!("This is the body: \n{}", body_text);
        let response: SubsonicArtistResponse<Discography> =
            self.client.get(url).send().await?.json().await?;
        let disc = response.subsonic_response.artist.album;
        for album in disc {
            albums.push(Album {
                name: album.name,
                id: album.id,
                artist: album.artist,
            });
        }

        Ok(albums)
    }
    pub async fn scrobble(&self, track: &Track) -> Result<()> {
        let mut scrobble_url = Url::parse(&format!("{}/rest/scrobble", self.base_url))?;
        let mut params = self.get_auth_params();
        params.push(("id", track.id.clone()));
        for (key, value) in params {
            scrobble_url.query_pairs_mut().append_pair(key, &value);
        }
        let res: SubsonicResponseEmpty = self.client.get(scrobble_url).send().await?.json().await?;
        if res.subsonic_response.status == "ok" {
            Ok(())
        } else {
            Err(anyhow::anyhow!(format!(
                "Unable to scrobble for track: {:?}",
                track
            )))
        }
    }
    pub async fn get_songs_in_album(&self, album: &Album) -> Result<Vec<Track>> {
        let mut tracks = Vec::new();
        let mut album_url = Url::parse(&format!("{}/rest/getAlbum", self.base_url))?;
        let mut cover_art = Url::parse(&format!("{}/rest/getCoverArt", self.base_url))?;
        let mut params = self.get_auth_params();
        params.push(("id", album.id.clone()));
        for (key, value) in params {
            album_url.query_pairs_mut().append_pair(key, &value);
            cover_art.query_pairs_mut().append_pair(key, &value);
        }
        let album_response: SubsonicResponse<GetAlbumResponse> =
            self.client.get(album_url).send().await?.json().await?;

        for song in album_response.subsonic_response.data.album.song {
            let length = song.duration.unwrap_or(0) * 1_000_000;
            tracks.push(Track {
                id: song.id,
                title: song.title,
                artist: song.artist,
                album_artist: song.display_album_artist,
                album: song.album,
                cover_art: Some(cover_art.to_string()),
                duration: length,
                track_number: song.track_number,
                // genre: song.genre,
                play_count: song.play_count,
                genres: song.genres.iter().map(|f| f.name.clone()).collect(),
            });
        }
        Ok(tracks)
    }
    pub async fn get_all_songs(&self) -> Result<Vec<Track>> {
        let albums = self.get_all_albums().await?;
        let mut tracks = Vec::new();
        for album in albums {
            let songs = self.get_songs_in_album(&album).await?;
            tracks.extend(songs);
        }
        Ok(tracks)
    }
    pub fn get_stream_url(&self, id: &str) -> Result<String> {
        let mut url = Url::parse(&format!("{}/rest/stream", self.base_url))?;
        let mut params = self.get_auth_params();
        params.push(("id", id.to_string()));
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, &value);
        }
        Ok(url.to_string())
    }
}
