use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    app::{ActiveSection, ActiveTab, InputMode},
    config::SearchMode,
};

use super::App;
impl App {
    pub fn start_inline_search(&mut self) {
        self.input_mode = InputMode::InlineSearch;
        self.search_query.clear();
    }
    pub fn exit_inline_search(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
    }
    pub fn inline_search_input(&mut self, c: char) {
        if self.input_mode == InputMode::InlineSearch {
            self.search_query.push(c);
            self.jump_to_inline_match();
        }
    }
    pub fn inline_search_backspace(&mut self) {
        if self.input_mode == InputMode::InlineSearch {
            self.search_query.pop();
            self.jump_to_inline_match();
        }
    }

    pub fn jump_to_inline_match(&mut self) {
        if self.search_query.is_empty() {
            return;
        }
        let query = self.search_query.to_lowercase();
        match self.active_section {
            ActiveSection::Queue => {
                if let Some(idx) = self.queue_tab.data.iter().position(|t| {
                    t.title.to_lowercase().contains(&query)
                        || t.artist.to_lowercase().contains(&query)
                }) {
                    self.queue_tab.select(idx);
                }
            }
            ActiveSection::Others => match self.active_tab {
                ActiveTab::Songs => {
                    if let Some(idx) = self.tracks_tab.data.iter().position(|t| {
                        t.title.to_lowercase().contains(&query)
                            || t.artist.to_lowercase().contains(&query)
                    }) {
                        self.tracks_tab.select(idx);
                    }
                }
                ActiveTab::Favorites => {
                    if let Some(idx) = self.favorite_tab.data.iter().position(|a| {
                        a.title.to_lowercase().contains(&query)
                            || a.artist.to_lowercase().contains(&query)
                    }) {
                        self.favorite_tab.select(idx);
                    }
                }
                ActiveTab::Artists => {
                    if let Some(idx) = self
                        .artist_tab
                        .data
                        .iter()
                        .position(|a| a.name.to_lowercase().contains(&query))
                    {
                        self.artist_tab.select(idx);
                    }
                }
                ActiveTab::Albums => {
                    if let Some(idx) = self.album_tab.data.iter().position(|a| {
                        a.name.to_lowercase().contains(&query)
                            || a.artist.to_lowercase().contains(&query)
                    }) {
                        self.album_tab.select(idx);
                    }
                }
                ActiveTab::Playlist => {
                    if let Some(idx) = self
                        .playlist_tab
                        .data
                        .iter()
                        .position(|p| p.name.to_lowercase().contains(&query))
                    {
                        self.playlist_tab.select(idx);
                    }
                }
                ActiveTab::Search => {}
            },
        }
    }
    pub async fn handle_inline_search_input(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.exit_inline_search();
            }
            KeyCode::Backspace => {
                self.inline_search_backspace();
            }
            KeyCode::Char(c) => {
                self.inline_search_input(c);
            }
            _ => {}
        }
        Ok(false)
    }
    pub fn enter_search_mode(&mut self) {
        self.input_mode = InputMode::Search;
        self.active_tab = ActiveTab::Search;
        self.search_query.clear();
        self.search_tab.index = 0;
        self.search_tab.clear();
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
        self.search_tab.index = 0;
        self.search_tab.clear();
    }
    pub async fn perform_search(&mut self) -> Result<()> {
        if self.search_query.is_empty() {
            self.search_tab.clear();
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
        self.search_tab.index = 0;
        if !self.search_tab.data.is_empty() {
            self.search_tab.select(0);
        } else {
            self.search_tab.clear();
        }
        Ok(())
    }
    /// Perform local fuzzy search on loaded tracks
    fn perform_local_search(&mut self) {
        let results = self
            .search_engine
            .search(&self.search_query, &self.tracks_tab.data);
        self.search_tab.data = results.into_iter().map(|r| r.track).collect();
    }

    /// Perform remote search using subsonic api. This is useful for when you have a proxy in
    /// between to search for missing songs
    async fn perform_remote_search(&mut self) -> Result<()> {
        self.search_tab.data = self.subsonic_client.search(&self.search_query).await?;
        Ok(())
    }
    pub async fn handle_search_input(&mut self, key: KeyEvent) -> Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.exit_search_mode();
            }
            KeyCode::Enter => {
                if !self.search_query.is_empty() {
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
                self.last_search_keystroke = Some(std::time::Instant::now());
                // self.perform_search().await?;
            }
            KeyCode::Backspace => {
                self.search_backspace();
                self.last_search_keystroke = Some(std::time::Instant::now());
                // self.perform_search().await?;
            }
            _ => {}
        }
        Ok(false)
    }
}
