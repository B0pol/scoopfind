pub mod api;
pub mod app;
pub mod models;
pub mod ui;

use std::io::stdout;
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use crate::api::ApiClient;
use crate::app::{App, Focus};
use crate::models::ScoopPackage;

enum WorkerRequest {
    Search { query: String, top: usize, skip: usize },
}

struct WorkerResponse {
    query: String,
    result: anyhow::Result<Vec<ScoopPackage>>,
    key_refreshed: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup panic hook to always restore terminal on abnormal exit
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // 2. Initialize terminal
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // 3. Channels for non-blocking background API search
    let (req_tx, mut req_rx) = mpsc::unbounded_channel::<WorkerRequest>();
    let (resp_tx, mut resp_rx) = mpsc::unbounded_channel::<WorkerResponse>();

    // 4. Spawn background API worker
    tokio::spawn(async move {
        let mut api_client = ApiClient::new();
        while let Some(req) = req_rx.recv().await {
            match req {
                WorkerRequest::Search { query, top, skip } => {
                    let prev_refreshed = api_client.key_refreshed;
                    let result = api_client.search(&query, top, skip).await;
                    let key_refreshed = api_client.key_refreshed && !prev_refreshed;
                    let _ = resp_tx.send(WorkerResponse {
                        query,
                        result,
                        key_refreshed,
                    });
                }
            }
        }
    });

    let mut app = App::new();

    // 5. Main TUI Event Loop
    while !app.should_quit {
        // Draw TUI
        terminal.draw(|f| ui::draw(f, &app))?;

        // Poll for terminal events with 30ms timeout for smooth responsiveness
        if event::poll(Duration::from_millis(30))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    // Global exit shortcut: requires double Ctrl+C to quit
                    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                        app.handle_ctrl_c();
                        continue;
                    }

                    match key.code {
                        KeyCode::Esc => {
                            app.handle_escape();
                        }
                        KeyCode::Tab => {
                            app.focus = match app.focus {
                                Focus::SearchInput => {
                                    if !app.results.is_empty() {
                                        Focus::ResultsList
                                    } else {
                                        Focus::SearchInput
                                    }
                                }
                                Focus::ResultsList => Focus::DetailPane,
                                Focus::DetailPane => Focus::SearchInput,
                            };
                        }
                        _ => match app.focus {
                            Focus::SearchInput => handle_search_input_keys(&mut app, key.code, &req_tx),
                            Focus::ResultsList => handle_results_list_keys(&mut app, key.code),
                            Focus::DetailPane => handle_detail_pane_keys(&mut app, key.code),
                        },
                    }
                }
            }
        }

        // Receive any finished searches from background worker
        while let Ok(response) = resp_rx.try_recv() {
            if response.query == app.query.trim() {
                match response.result {
                    Ok(packages) => app.set_results(packages, response.key_refreshed),
                    Err(e) => app.set_error(format!("{:#}", e)),
                }
            }
        }
    }

    // 6. Graceful cleanup and exit
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

fn handle_search_input_keys(
    app: &mut App,
    code: KeyCode,
    req_tx: &mpsc::UnboundedSender<WorkerRequest>,
) {
    match code {
        KeyCode::Char(c) => {
            app.insert_char(c);
        }
        KeyCode::Backspace => {
            app.delete_char();
        }
        KeyCode::Left => {
            app.move_cursor_left();
        }
        KeyCode::Right => {
            app.move_cursor_right();
        }
        KeyCode::Down => {
            if !app.results.is_empty() {
                app.focus = Focus::ResultsList;
            }
        }
        KeyCode::Enter => {
            let trimmed = app.query.trim().to_string();
            // Exit commands
            if trimmed.eq_ignore_ascii_case("/quit") || trimmed.eq_ignore_ascii_case("/exit") {
                app.should_quit = true;
                return;
            }
            if !trimmed.is_empty() {
                app.is_loading = true;
                app.status_message = format!("Searching for \"{}\" in Scoop index...", trimmed);
                let _ = req_tx.send(WorkerRequest::Search {
                    query: trimmed,
                    top: 50,
                    skip: 0,
                });
            }
        }
        _ => {}
    }
}

fn handle_results_list_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.select_previous();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_next();
        }
        KeyCode::Home | KeyCode::Char('g') => {
            app.select_first();
        }
        KeyCode::End | KeyCode::Char('G') => {
            app.select_last();
        }
        KeyCode::Char('/') | KeyCode::Char('i') => {
            app.focus = Focus::SearchInput;
        }
        _ => {}
    }
}

fn handle_detail_pane_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('/') | KeyCode::Char('i') => {
            app.focus = Focus::SearchInput;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.focus = Focus::ResultsList;
        }
        _ => {}
    }
}
