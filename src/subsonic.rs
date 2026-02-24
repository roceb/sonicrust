use crate::app;
use crate::app::{Album, Artist, Playlists, Track};
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

#[derive(Deserialize, Debug)]
struct SubsonicResponse<T> {
    #[serde(rename = "subsonic-response")]
    subsonic_response: SubsonicResponseInner<T>,
}

#[derive(Deserialize, Debug)]
struct SubsonicResponseInner<T> {
    status: String,
    #[serde(flatten)]
    data: Option<T>,
}

impl<T> SubsonicResponse<T> {
    fn is_ok(&self) -> bool {
        self.subsonic_response.status == "ok"
    }
    fn into_data(self) -> Result<T> {
        if self.is_ok() {
            self.subsonic_response
                .data
                .context("Response OK but no data")
        } else {
            Err(anyhow::anyhow!("Subsonic error response"))
        }
    }
}

#[derive(Deserialize, Debug)]
struct StarredData {
    starred2: Favorites,
}
#[derive(Deserialize, Debug)]
struct Favorites {
    song: Option<Vec<Song>>,
    _album: Option<Vec<AlbumInfo>>,
    _artist: Option<Vec<ArtistInfo>>,
}

#[derive(Deserialize, Debug)]
struct PlaylistsData {
    playlists: PlaylistWrapper,
}

#[derive(Deserialize, Debug)]
struct PlaylistData {
    playlist: Playlist,
}
#[derive(Deserialize, Debug)]
struct ArtistData {
    artist: Discography,
}

#[derive(Deserialize, Debug)]
struct PlaylistWrapper {
    #[serde(default)]
    playlist: Vec<PlaylistInfo>,
}

#[derive(Deserialize, Debug)]
struct PlaylistInfo {
    id: String,
    name: String,
    #[serde(rename = "songCount")]
    song_count: i32,
    duration: i64,
}

#[derive(Deserialize, Debug)]
struct Playlist {
    #[serde(default)]
    entry: Vec<Song>,
}

#[derive(Deserialize)]
struct GetArtistsListResponse {
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
}
#[derive(Deserialize, Debug)]
struct ArtistInfo {
    id: String,
    name: String,
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
#[derive(Deserialize, Debug)]
struct GetAlbumResponse {
    album: AlbumDetail,
}
#[derive(Deserialize, Debug)]
struct AlbumDetail {
    song: Vec<Song>,
}
#[derive(Deserialize, Debug)]
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
    #[serde(rename = "playCount")]
    play_count: Option<i32>,
    #[serde(rename = "displayAlbumArtist")]
    display_album_artist: Option<String>,
    genres: Vec<Genres>,
}

impl Song {
    fn into_track(self, cover_art_url: String) -> Track {
        Track {
            id: self.id,
            title: self.title,
            artist: self.artist,
            album_artist: self.display_album_artist,
            album: self.album,
            cover_art: Some(cover_art_url),
            duration: self.duration.unwrap_or(0) * 1_000_000,
            track_number: self.track_number,
            play_count: self.play_count,
            genres: self.genres.iter().map(|f| f.name.clone()).collect(),
        }
    }
}

impl SubsonicClient {
    fn build_cover_art_url(&self, id: &str) -> Result<String> {
        let mut url = Url::parse(&format!("{}/rest/getCoverArt", self.base_url))?;
        let mut params = self.get_auth_params();
        params.push(("id", id.to_string()));
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, &value);
        }
        Ok(url.to_string())
    }
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
    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        extra_params: Vec<(&str, String)>,
    ) -> Result<T> {
        let mut url = Url::parse(&format!("{}/rest/{}", self.base_url, endpoint))?;
        let mut params = self.get_auth_params();
        params.extend(extra_params);
        for (key, value) in params {
            url.query_pairs_mut().append_pair(key, &value);
        }
        let res: SubsonicResponse<T> = self.client.get(url).send().await?.json().await?;
        res.into_data()
    }
    fn songs_to_tracks(&self, songs: Vec<Song>) -> Result<Vec<Track>> {
        songs
            .into_iter()
            .map(|song| {
                let cover_art = self.build_cover_art_url(&song.id)?;
                Ok(song.into_track(cover_art))
            })
            .collect()
    }
    pub async fn search(&self, search: &str) -> Result<Vec<Track>> {
        if search.is_empty() {
            return Ok(Vec::new());
        }
        let data: SearchResult3 = self
            .get("search3", vec![("query", search.to_string())])
            .await?;
        self.songs_to_tracks(data.search_result.song)
    }
    pub async fn get_all_albums(&self) -> Result<Vec<Album>> {
        // TODO: Add paginating to help with big libraries
        let data: GetAlbumListResponse = self
            .get(
                "getAlbumList2",
                vec![
                    ("type", "alphabeticalByArtist".to_string()),
                    ("size", "500".to_string()),
                ],
            )
            .await?;
        let mut albums = Vec::new();
        for album in data.album_list.album {
            albums.push(Album {
                id: album.id,
                name: album.name,
                artist: album.artist,
            });
        }
        Ok(albums)
    }
    pub async fn get_all_favorites(&self) -> Result<Vec<Track>> {
        let data: StarredData = self.get("getStarred2", vec![]).await?;
        self.songs_to_tracks(data.starred2.song.unwrap_or_default())
    }
    pub async fn get_all_artists(&self) -> Result<Vec<Artist>> {
        let mut artists = Vec::new();
        let data: GetArtistsListResponse = self.get("getArtists", vec![]).await?;
        for letter_artist in data.artists.artist_index {
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
    pub async fn get_playlists(&self) -> Result<Vec<app::Playlists>> {
        let data: PlaylistsData = self.get("getPlaylists", vec![]).await?;
        let mut playlists: Vec<app::Playlists> = Vec::new();
        for play in data.playlists.playlist {
            playlists.push(app::Playlists {
                id: play.id,
                song_count: play.song_count,
                name: play.name,
                duration: play.duration,
            });
        }
        Ok(playlists)
    }
    pub async fn get_songs_from_playlist(&self, playlist: &Playlists) -> Result<Vec<Track>> {
        let data: PlaylistData = self
            .get("getPlaylist", vec![("id", playlist.id.clone())])
            .await?;
        self.songs_to_tracks(data.playlist.entry)
    }
    pub async fn get_artist_albums(&self, artist: &Artist) -> Result<Vec<Album>> {
        let data: ArtistData = self
            .get("getArtist", vec![("id", artist.id.clone())])
            .await?;
        let mut albums: Vec<Album> = Vec::new();
        let disc = data.artist.album;
        for album in disc {
            albums.push(Album {
                name: album.name,
                id: album.id,
                artist: album.artist,
            });
        }

        Ok(albums)
    }
    pub async fn scrobble(&self, track: &Track, submission: bool) -> Result<()> {
        let _: SubsonicResponseInner<()> = self.get("scrobble", vec![("id", track.id.clone()),("submission", submission.to_string())]).await?;
        Ok(())
    }
    pub async fn get_songs_in_album(&self, album: &Album) -> Result<Vec<Track>> {
        let data :GetAlbumResponse= self.get("getAlbum", vec![("id", album.id.clone())]).await?;
        self.songs_to_tracks(data.album.song)
    }
    pub async fn get_all_songs(&self) -> Result<Vec<Track>> {
        let albums = self.get_all_albums().await?;
        let futures = albums.iter().map(|a| self.get_songs_in_album(a));
        let results = futures::future::join_all(futures).await;
        let a = results.into_iter().flat_map(|r| r.unwrap_or_default()).collect::<Vec<_>>();
        Ok(a)
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
