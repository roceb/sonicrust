use crate::app::Track;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub struct SearchEngine {
    matcher: SkimMatcherV2,
    threshold: i64,
    max_results: usize,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub track: Track,
    pub score: i64,
    pub _match_field: MatchField,
}
#[derive(Clone, Debug, PartialEq)]
pub enum MatchField {
    Title,
    Artist,
    Album,
    Multiple,
}

impl SearchEngine {
    pub fn new(threshold: i64, max_results: usize) -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            threshold,
            max_results,
        }
    }
    pub fn search(&self, query: &str, tracks: &[Track]) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }
        let query_formatted = query.to_lowercase();
        let mut results: Vec<SearchResult> = Vec::new();
        for track in tracks {
            let mut best_score: i64 = 0;
            let mut match_field = MatchField::Title;
            if let Some(score) = self
                .matcher
                .fuzzy_match(&track.title.to_lowercase(), &query_formatted)
            && score > best_score {
                    best_score = score;
                    match_field = MatchField::Title;
            }
            if let Some(score) = self
                .matcher
                .fuzzy_match(&track.artist.to_lowercase(), &query_formatted)
            {
                if score > best_score {
                    best_score = score;
                    match_field = MatchField::Artist;
                } else if score == best_score && best_score > 0 {
                    match_field = MatchField::Multiple;
                }
            }
            if let Some(score) = self
                .matcher
                .fuzzy_match(&track.album.to_lowercase(), &query_formatted)
            {
                if score > best_score {
                    best_score = score;
                    match_field = MatchField::Album;
                } else if score == best_score && best_score > 0 {
                    match_field = MatchField::Multiple;
                }
            }
            let combined = format!("{} {}", track.artist, track.title).to_lowercase();
            if let Some(score) = self.matcher.fuzzy_match(&combined, &query_formatted)
                && score > best_score {
                    best_score = score;
                    match_field = MatchField::Multiple;
            }
            if best_score >= self.threshold {
                results.push(SearchResult {
                    track: track.clone(),
                    score: best_score,
                    _match_field: match_field,
                });
            }
        }
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(self.max_results);
        results
    }
    // same as above but is faster at the expense that it isn't fuzzy
    pub fn _search_exact(&self, query: &str, tracks: &[Track]) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }
        let query_formatted = query.to_lowercase();
        let mut results: Vec<SearchResult> = Vec::new();

        for track in tracks {
            let title_match = track.title.to_lowercase().contains(&query_formatted);
            let artist_match = track.artist.to_lowercase().contains(&query_formatted);
            let album_match = track.album.to_lowercase().contains(&query_formatted);
            if title_match || artist_match || album_match {
                let match_field = if title_match && (artist_match || album_match) {
                    MatchField::Multiple
                } else if title_match {
                    MatchField::Title
                } else if artist_match {
                    MatchField::Artist
                } else {
                    MatchField::Album
                };
                let score = if title_match { 100 } else { 0 }
                    + if artist_match { 80 } else { 0 }
                    + if album_match { 60 } else { 0 };
                results.push(SearchResult {
                    track: track.clone(),
                    score,
                    _match_field: match_field,
                });
            }
        }
        results.sort_by(|a, b| b.score.cmp(&a.score));
        results.truncate(self.max_results);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_track(title: String, artist: String, album: String) -> Track {
        Track {
            id: "test".to_string(),
            title,
            artist,
            album,
            cover_art: String::new(),
            duration: 0,
        }
    }
    #[test]
    fn test_fuzzy_search_title() {
        let engine = SearchEngine::new(50, 100);
        let tracks = vec![
            create_test_track(
                "Bohemian Rhapsody".to_string(),
                "Queen".to_string(),
                "A Night at the Opera".to_string(),
            ),
            create_test_track(
                "The Real Slim Shady".to_string(),
                "Emienem".to_string(),
                "The Slim LP".to_string(),
            ),
            create_test_track(
                "Rolling in the Deep".to_string(),
                "Adele".to_string(),
                "21".to_string(),
            ),
        ];
        let results = engine.search("real", &tracks);
        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().track.title, "The Real Slim Shady")
    }
    #[test]
    fn test_fuzzy_artist_title() {
        let engine = SearchEngine::new(50, 100);
        let tracks = vec![
            create_test_track(
                "Bohemian Rhapsody".to_string(),
                "Queen".to_string(),
                "A Night at the Opera".to_string(),
            ),
            create_test_track(
                "The Real Slim Shady".to_string(),
                "Emienem".to_string(),
                "The Slim LP".to_string(),
            ),
            create_test_track(
                "Rolling in the Deep".to_string(),
                "Adele".to_string(),
                "21".to_string(),
            ),
        ];
        let results = engine.search("queen", &tracks);
        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().track.title, "Bohemian Rhapsody")
    }
    #[test]
    fn test_fuzzy_search_album() {
        let engine = SearchEngine::new(50, 100);
        let tracks = vec![
            create_test_track(
                "Bohemian Rhapsody".to_string(),
                "Queen".to_string(),
                "A Night at the Opera".to_string(),
            ),
            create_test_track(
                "The Real Slim Shady".to_string(),
                "Emienem".to_string(),
                "The Slim LP".to_string(),
            ),
            create_test_track(
                "Rolling in the Deep".to_string(),
                "Adele".to_string(),
                "21".to_string(),
            ),
        ];
        let results = engine.search("21", &tracks);
        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().track.title, "Rolling in the Deep")
    }
}
