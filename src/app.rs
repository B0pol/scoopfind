use std::time::Instant;
use crate::models::ScoopPackage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    SearchInput,
    ResultsList,
    DetailPane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmButton {
    Install,
    Cancel,
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
    /// Set to true when the user requests to quit (double Ctrl+C or /quit or /exit)
    pub should_quit: bool,
    /// Active UI focus
    pub focus: Focus,
    /// Current search page (for pagination)
    pub page: usize,
    /// Notification if Milestone 5 key refresh occurred
    pub key_refreshed: bool,
    /// Timestamp of last Esc press for double-tap detection
    pub last_esc_time: Option<Instant>,
    /// Timestamp of last Ctrl+C press for double-tap detection
    pub last_ctrl_c_time: Option<Instant>,
    /// Package currently pending confirmation in the install modal dialog
    pub confirming_install: Option<ScoopPackage>,
    /// Currently focused button in the confirmation modal
    pub confirm_button: ConfirmButton,
    /// Package queued for installation after user confirmation
    pub pending_install: Option<ScoopPackage>,
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
            last_esc_time: None,
            last_ctrl_c_time: None,
            confirming_install: None,
            confirm_button: ConfirmButton::Install,
            pending_install: None,
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

    /// Handles Esc press:
    /// - If outside search bar: switches focus to search bar.
    /// - If double Esc (within 500ms): clears the search query.
    pub fn handle_escape(&mut self) {
        let now = Instant::now();
        let is_double_esc = self
            .last_esc_time
            .map(|t| now.duration_since(t).as_millis() <= 500)
            .unwrap_or(false);

        self.last_esc_time = Some(now);

        if self.focus != Focus::SearchInput {
            self.focus = Focus::SearchInput;
        }

        if is_double_esc {
            self.clear_query();
            self.status_message = "Search query cleared.".to_string();
        }
    }

    /// Handles Ctrl+C press:
    /// - First press: shows warning "Press Ctrl+C again to quit"
    /// - Second press within 1500ms: sets should_quit = true
    pub fn handle_ctrl_c(&mut self) {
        let now = Instant::now();
        let is_double_ctrl_c = self
            .last_ctrl_c_time
            .map(|t| now.duration_since(t).as_millis() <= 1500)
            .unwrap_or(false);

        if is_double_ctrl_c {
            self.should_quit = true;
        } else {
            self.last_ctrl_c_time = Some(now);
            self.status_message = "Press Ctrl+C again to quit.".to_string();
        }
    }

    /// Opens the confirmation modal for the currently selected package.
    pub fn prompt_install(&mut self) {
        if let Some(pkg) = self.selected_package().cloned() {
            self.status_message = format!("Install '{}'? Press <Enter> to confirm or <Esc> to cancel.", pkg.name);
            self.confirming_install = Some(pkg);
            self.confirm_button = ConfirmButton::Install;
        }
    }

    /// Confirms or cancels based on the currently selected button in the modal.
    pub fn confirm_install(&mut self) {
        if let Some(pkg) = self.confirming_install.take() {
            match self.confirm_button {
                ConfirmButton::Install => {
                    self.pending_install = Some(pkg);
                }
                ConfirmButton::Cancel => {
                    self.status_message = "Installation cancelled.".to_string();
                }
            }
        }
    }

    /// Cancels installation and closes the modal.
    pub fn cancel_install(&mut self) {
        self.confirming_install = None;
        self.status_message = "Installation cancelled.".to_string();
    }

    /// Toggles between the Install and Cancel buttons in the modal.
    pub fn toggle_confirm_button(&mut self) {
        self.confirm_button = match self.confirm_button {
            ConfirmButton::Install => ConfirmButton::Cancel,
            ConfirmButton::Cancel => ConfirmButton::Install,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::models::PackageMetadata;

    fn sample_package(name: &str) -> ScoopPackage {
        ScoopPackage {
            id: name.to_string(),
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test description".to_string()),
            homepage: None,
            license: None,
            notes: None,
            metadata: PackageMetadata {
                repository: "https://github.com/ScoopInstaller/Main".to_string(),
                official_repository: true,
                repository_stars: None,
                file_path: format!("bucket/{}.json", name),
                committed: None,
                sha: None,
            },
        }
    }

    #[test]
    fn test_escape_focus_and_double_escape_clear() {
        let mut app = App::new();
        app.focus = Focus::ResultsList;
        app.query = "ripgrep".to_string();
        app.cursor_position = 7;

        // First Esc: returns to SearchInput, does NOT clear query
        app.handle_escape();
        assert_eq!(app.focus, Focus::SearchInput);
        assert_eq!(app.query, "ripgrep");

        // Second Esc immediately: double Esc clears query
        app.handle_escape();
        assert_eq!(app.query, "");
        assert_eq!(app.cursor_position, 0);
        assert_eq!(app.status_message, "Search query cleared.");
    }

    #[test]
    fn test_ctrl_c_double_tap_quit() {
        let mut app = App::new();

        // First Ctrl+C: prompts without quitting
        app.handle_ctrl_c();
        assert!(!app.should_quit);
        assert_eq!(app.status_message, "Press Ctrl+C again to quit.");

        // Second Ctrl+C immediately: sets should_quit to true
        app.handle_ctrl_c();
        assert!(app.should_quit);
    }

    #[test]
    fn test_ctrl_c_timeout_resets() {
        let mut app = App::new();

        // First Ctrl+C
        app.handle_ctrl_c();
        assert!(!app.should_quit);

        // Simulate elapsed time > 1500ms
        app.last_ctrl_c_time = Some(Instant::now() - Duration::from_millis(2000));

        // Another Ctrl+C after timeout: does NOT quit, prompts again
        app.handle_ctrl_c();
        assert!(!app.should_quit);
    }

    #[test]
    fn test_install_confirmation_flow() {
        let mut app = App::new();
        app.results = vec![sample_package("neovim")];
        app.selected_index = 0;

        // Prompt install opens modal
        app.prompt_install();
        assert!(app.confirming_install.is_some());
        assert_eq!(app.confirming_install.as_ref().unwrap().name, "neovim");
        assert_eq!(app.confirm_button, ConfirmButton::Install);

        // Confirming with button Install queues for execution
        app.confirm_install();
        assert!(app.confirming_install.is_none());
        assert!(app.pending_install.is_some());
        assert_eq!(app.pending_install.as_ref().unwrap().name, "neovim");
    }

    #[test]
    fn test_install_cancellation_flow() {
        let mut app = App::new();
        app.results = vec![sample_package("ripgrep")];
        app.selected_index = 0;

        app.prompt_install();
        assert!(app.confirming_install.is_some());

        // Cancel closes modal without pending install
        app.cancel_install();
        assert!(app.confirming_install.is_none());
        assert!(app.pending_install.is_none());
        assert_eq!(app.status_message, "Installation cancelled.");
    }

    #[test]
    fn test_toggle_confirm_button() {
        let mut app = App::new();
        app.results = vec![sample_package("git")];
        app.prompt_install();

        assert_eq!(app.confirm_button, ConfirmButton::Install);
        app.toggle_confirm_button();
        assert_eq!(app.confirm_button, ConfirmButton::Cancel);

        // Confirming while on Cancel button cancels installation
        app.confirm_install();
        assert!(app.confirming_install.is_none());
        assert!(app.pending_install.is_none());
        assert_eq!(app.status_message, "Installation cancelled.");
    }
}
