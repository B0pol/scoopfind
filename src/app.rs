use crate::models::ScoopPackage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    SearchInput,
    ResultsList,
    DetailPane,
}

pub struct App {
    /// Current search query string typed by the user
    pub query: String,
    /// Byte/character cursor position within the query string
    pub cursor_position: usize,
    /// Loaded search results from the Azure API
    pub results: Vec<ScoopPackage>,
    /// Index of the currently highlighted result in the list
    pub selected_index: usize,
    /// Indicates whether a background HTTP search is currently running
    pub is_loading: bool,
    /// Informational message displayed in the footer
    pub status_message: String,
    /// Set to true when the user requests to quit (Esc, Ctrl+C, or 'q' outside input)
    pub should_quit: bool,
    /// Active UI focus
    pub focus: Focus,
    /// Current search page (for pagination)
    pub page: usize,
    /// Notification if Milestone 5 key refresh occurred
    pub key_refreshed: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            cursor_position: 0,
            results: Vec::new(),
            selected_index: 0,
            is_loading: false,
            status_message: "Ready. Type a package name and press <Enter> to search.".to_string(),
            should_quit: false,
            focus: Focus::SearchInput,
            page: 1,
            key_refreshed: false,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.query.insert(self.cursor_position, c);
        self.cursor_position += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_position > 0 && !self.query.is_empty() {
            self.cursor_position -= 1;
            self.query.remove(self.cursor_position);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_position < self.query.len() {
            self.cursor_position += 1;
        }
    }

    pub fn clear_query(&mut self) {
        self.query.clear();
        self.cursor_position = 0;
    }

    pub fn select_next(&mut self) {
        if !self.results.is_empty() && self.selected_index + 1 < self.results.len() {
            self.selected_index += 1;
        }
    }

    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn select_first(&mut self) {
        self.selected_index = 0;
    }

    pub fn select_last(&mut self) {
        if !self.results.is_empty() {
            self.selected_index = self.results.len() - 1;
        }
    }

    pub fn selected_package(&self) -> Option<&ScoopPackage> {
        self.results.get(self.selected_index)
    }

    pub fn set_results(&mut self, packages: Vec<ScoopPackage>, refreshed_key: bool) {
        self.is_loading = false;
        self.results = packages;
        self.selected_index = 0;
        self.key_refreshed = refreshed_key;

        if self.results.is_empty() {
            self.status_message = format!("No packages found matching \"{}\"", self.query.trim());
        } else {
            let key_note = if refreshed_key {
                " [Auto-healed API key from scoop.sh]"
            } else {
                ""
            };
            self.status_message = format!(
                "Found {} packages for \"{}\"{}",
                self.results.len(),
                self.query.trim(),
                key_note
            );
            self.focus = Focus::ResultsList;
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.is_loading = false;
        self.status_message = format!("Error: {}", error);
    }
}
