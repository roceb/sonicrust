use crate::app::{ActiveSection, ActiveTab, InputMode};

use super::App;
macro_rules! navigate_list {
    ($state:expr, $forward:expr) => {
        let len = $state.data.len();
        if len > 0 {
            let i = if $forward {
                $state.state.selected().map_or(0, |s| (s + 1) % len)
            } else {
                $state
                    .state
                    .selected()
                    .map_or(len - 1, |s| if s == 0 { len - 1 } else { s - 1 })
            };
            $state.index = i;
            $state.state.select(Some(i));
        } else {
            $state.state.select(None)
        }
    };
}

impl App {
    pub fn select_tab(&mut self, tab: ActiveTab) {
        match self.active_tab {
            // ActiveTab::Queue => self.queue_state.select(None),
            ActiveTab::Songs => self.tracks_tab.clear(),
            ActiveTab::Playlist => self.playlist_tab.clear(),
            ActiveTab::Artists => self.artist_tab.clear(),
            ActiveTab::Albums => self.album_tab.clear(),
            ActiveTab::Favorites => self.favorite_tab.clear(),
            ActiveTab::Search => {
                self.search_tab.clear();
                self.input_mode = InputMode::Normal;
            }
        }

        self.active_tab = tab.clone();

        // Initialize new tab state
        match tab {
            ActiveTab::Playlist if !self.playlist_tab.data.is_empty() => {
                self.playlist_tab.current();
            }
            ActiveTab::Songs if !self.tracks_tab.data.is_empty() => {
                self.tracks_tab.current();
            }
            ActiveTab::Artists if !self.artist_tab.data.is_empty() => {
                self.artist_tab.current();
            }
            ActiveTab::Favorites if !self.favorite_tab.data.is_empty() => {
                self.favorite_tab.current();
            }
            ActiveTab::Albums if !self.album_tab.data.is_empty() => {
                self.album_tab.current();
            }
            ActiveTab::Search if !self.search_tab.data.is_empty() => {
                self.search_tab.current();
            }
            _ => {}
        }
    }
    pub fn next_tab(&mut self) {
        self.active_section = match self.active_section {
            ActiveSection::Queue => ActiveSection::Others,
            ActiveSection::Others => ActiveSection::Queue,
        };
        match self.active_section {
            ActiveSection::Queue => {
                if !self.queue_tab.data.is_empty() {
                    self.queue_tab.current();
                }
            }
            ActiveSection::Others => match self.active_tab {
                ActiveTab::Songs => {
                    self.artist_tab.select(self.tracks_tab.index);
                }
                ActiveTab::Favorites => {
                    self.favorite_tab.current();
                }
                ActiveTab::Playlist => {
                    self.playlist_tab.current();
                }
                ActiveTab::Artists => {
                    self.album_tab.select(self.artist_tab.index);
                }
                ActiveTab::Albums => {
                    self.tracks_tab.select(self.album_tab.index);
                }
                ActiveTab::Search => {
                    self.tracks_tab.select(self.search_tab.index);
                }
            },
        };
    }
    pub fn previous_tab(&mut self) {
        self.next_tab();
    }
    pub fn next_item_in_tab(&mut self) {
        match self.active_section {
            ActiveSection::Queue => {
                navigate_list!(self.queue_tab, true);
            }
            ActiveSection::Others => match self.active_tab {
                ActiveTab::Search => {
                    if !self.search_tab.data.is_empty() {
                        navigate_list!(self.search_tab, true);
                    }
                }
                ActiveTab::Playlist => {
                    if !self.playlist_tab.data.is_empty() {
                        navigate_list!(self.playlist_tab, true);
                    }
                }
                ActiveTab::Songs => {
                    if !self.tracks_tab.data.is_empty() {
                        navigate_list!(self.tracks_tab, true);
                    }
                }
                ActiveTab::Favorites => {
                    if !self.favorite_tab.data.is_empty() {
                        navigate_list!(self.favorite_tab, true);
                    }
                }
                ActiveTab::Artists => {
                    if !self.artist_tab.data.is_empty() {
                        navigate_list!(self.artist_tab, true);
                    }
                }
                ActiveTab::Albums => {
                    if !self.album_tab.data.is_empty() {
                        navigate_list!(self.album_tab, true);
                    }
                }
            },
        }
    }
    pub fn previous_item_in_tab(&mut self) {
        match self.active_section {
            ActiveSection::Queue => {
                if !self.queue_tab.data.is_empty() {
                    navigate_list!(self.queue_tab, false);
                }
            }
            ActiveSection::Others => match self.active_tab {
                ActiveTab::Favorites => {
                    if !self.favorite_tab.data.is_empty() {
                        navigate_list!(self.favorite_tab, false);
                    }
                }
                ActiveTab::Playlist => {
                    if !self.playlist_tab.data.is_empty() {
                        navigate_list!(self.playlist_tab, false);
                    }
                }
                ActiveTab::Search => {
                    if !self.search_tab.data.is_empty() {
                        navigate_list!(self.search_tab, false);
                    }
                }
                ActiveTab::Songs => {
                    if !self.tracks_tab.data.is_empty() {
                        navigate_list!(self.tracks_tab, false);
                    }
                }
                ActiveTab::Artists => {
                    if !self.artist_tab.data.is_empty() {
                        navigate_list!(self.artist_tab, false);
                    }
                }
                ActiveTab::Albums => {
                    if !self.album_tab.data.is_empty() {
                        navigate_list!(self.album_tab, false);
                    }
                }
            },
        }
    }
}
