pub mod api;
pub mod app;
pub mod models;
pub mod ui;

use std::io::{stdout, Write};
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
use crate::app::{App, ConfirmButton, Focus};
use crate::models::ScoopPackage;

enum WorkerRequest {
    Search { query: String, top: usize, skip: usize },
}

struct WorkerResponse {
    query: String,
    result: anyhow::Result<Vec<ScoopPackage>>,
    key_refreshed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CliAction {
    Version,
    Help,
    Run { initial_query: Option<String> },
}

pub fn parse_args<I, T>(args: I) -> Result<CliAction, String>
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    let mut query_parts = Vec::new();
    let mut iter = args.into_iter().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_ref() {
            "-v" | "--version" => return Ok(CliAction::Version),
            "-h" | "--help" => return Ok(CliAction::Help),
            s if s.starts_with('-') => {
                return Err(format!("unexpected argument '{}'. For more information, try '--help'", s));
            }
            s => {
                query_parts.push(s.to_string());
            }
        }
    }

    let initial_query = if query_parts.is_empty() {
        None
    } else {
        Some(query_parts.join(" "))
    };

    Ok(CliAction::Run { initial_query })
}

#[tokio::main]
async fn main() -> Result<()> {
    match parse_args(std::env::args()) {
        Ok(CliAction::Version) => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Ok(CliAction::Help) => {
            println!(
                "{} {}\n\
                 Interactive TUI search for Scoop packages\n\n\
                 USAGE:\n    \
                 scoopfind [OPTIONS] [QUERY]\n\n\
                 ARGS:\n    \
                 <QUERY>    Initial search query\n\n\
                 OPTIONS:\n    \
                 -h, --help       Print help information\n    \
                 -v, --version    Print version information",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            );
            return Ok(());
        }
        Err(err) => {
            eprintln!("error: {}", err);
            std::process::exit(1);
        }
        Ok(CliAction::Run { initial_query }) => {
            run_tui(initial_query).await
        }
    }
}

async fn run_tui(initial_query: Option<String>) -> Result<()> {
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
    if let Some(query) = initial_query {
        let trimmed = query.trim().to_string();
        if !trimmed.is_empty() {
            app.query = trimmed.clone();
            app.cursor_position = app.query.chars().count();
            app.is_loading = true;
            app.status_message = format!("Searching for \"{}\" in Scoop index...", trimmed);
            let _ = req_tx.send(WorkerRequest::Search {
                query: trimmed,
                top: 50,
                skip: 0,
            });
        }
    }

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

                    // Handle keys when confirmation modal is open
                    if app.confirming_install.is_some() {
                        match key.code {
                            KeyCode::Enter => {
                                app.confirm_install();
                            }
                            KeyCode::Char('y') | KeyCode::Char('Y') => {
                                app.confirm_button = ConfirmButton::Install;
                                app.confirm_install();
                            }
                            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                                app.cancel_install();
                            }
                            KeyCode::Tab
                            | KeyCode::Left
                            | KeyCode::Right
                            | KeyCode::Char('h')
                            | KeyCode::Char('l') => {
                                app.toggle_confirm_button();
                            }
                            _ => {}
                        }
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

        // If a package was confirmed for installation, run the install interactively
        if let Some(pkg) = app.pending_install.take() {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            let install_cmd = pkg.install_command();
            let result = run_scoop_install(&pkg, &install_cmd);

            match result {
                Ok(install_res) => {
                    match install_res.action {
                        PostInstallAction::LaunchAndQuit => {
                            if let Err(err) = launch_installed_package(&pkg) {
                                eprintln!("Warning: failed to launch '{}': {:#}", pkg.name, err);
                            }
                            app.should_quit = true;
                            break;
                        }
                        PostInstallAction::Quit => {
                            app.should_quit = true;
                            break;
                        }
                        PostInstallAction::ReturnToApp => {
                            enable_raw_mode()?;
                            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                            terminal.clear()?;

                            // Drain any pending input events from the console
                            while event::poll(Duration::from_millis(0))? {
                                let _ = event::read()?;
                            }

                            if install_res.exit_code == 0 {
                                app.status_message = format!("Package '{}' installed successfully!", pkg.name);
                            } else {
                                app.status_message = format!(
                                    "Installation of '{}' completed with exit code {}.",
                                    pkg.name, install_res.exit_code
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    enable_raw_mode()?;
                    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                    terminal.clear()?;

                    while event::poll(Duration::from_millis(0))? {
                        let _ = event::read()?;
                    }

                    app.status_message = format!("Failed to run Scoop installation: {:#}", err);
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
        KeyCode::Enter => {
            app.prompt_install();
        }
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
        KeyCode::Enter => {
            app.prompt_install();
        }
        KeyCode::Char('/') | KeyCode::Char('i') => {
            app.focus = Focus::SearchInput;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.focus = Focus::ResultsList;
        }
        _ => {}
    }
}

#[cfg(windows)]
pub fn detect_powershell_binary() -> &'static str {
    detect_powershell_from_path_env(std::env::var_os("PATH"))
}

pub fn detect_powershell_from_path_env(path_var: Option<std::ffi::OsString>) -> &'static str {
    if let Some(path_var) = path_var {
        for dir in std::env::split_paths(&path_var) {
            if dir.join("pwsh.exe").is_file() {
                return "pwsh";
            }
        }
    }
    "powershell"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostInstallAction {
    LaunchAndQuit,
    ReturnToApp,
    Quit,
}

pub struct InstallResult {
    pub exit_code: i32,
    pub action: PostInstallAction,
}

pub fn parse_post_install_choice(choice: &str, is_success: bool) -> PostInstallAction {
    let trimmed = choice.trim();
    if is_success {
        match trimmed {
            "1" | "l" | "L" => PostInstallAction::LaunchAndQuit,
            "3" | "q" | "Q" => PostInstallAction::Quit,
            _ => PostInstallAction::ReturnToApp,
        }
    } else {
        match trimmed {
            "2" | "q" | "Q" => PostInstallAction::Quit,
            _ => PostInstallAction::ReturnToApp,
        }
    }
}

fn prompt_post_install_action(pkg_name: &str, exit_code: i32) -> PostInstallAction {
    println!();
    println!("------------------------------------------------------------");
    if exit_code == 0 {
        println!("Package '{}' installed successfully.", pkg_name);
        println!();
        println!("Options:");
        println!("  [1] Launch '{}' and quit", pkg_name);
        println!("  [2] Return to scoopfind (default)");
        println!("  [3] Quit scoopfind");
        println!();
        print!("Enter choice [1-3] (default 2): ");
        let _ = stdout().flush();

        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        parse_post_install_choice(&input, true)
    } else {
        println!("Scoop command exited with code {}.", exit_code);
        println!();
        println!("Options:");
        println!("  [1] Return to scoopfind (default)");
        println!("  [2] Quit scoopfind");
        println!();
        print!("Enter choice [1-2] (default 1): ");
        let _ = stdout().flush();

        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        parse_post_install_choice(&input, false)
    }
}

pub fn find_installed_executable(pkg_name: &str) -> Option<std::path::PathBuf> {
    let pkg_name_lower = pkg_name.to_lowercase();

    // 1. Check Scoop Start Menu shortcuts: %APPDATA%\Microsoft\Windows\Start Menu\Programs\Scoop Apps
    if let Ok(appdata) = std::env::var("APPDATA") {
        let shortcuts_dir = std::path::PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("Scoop Apps");

        if shortcuts_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&shortcuts_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("lnk"))
                    {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let stem_lower = stem.to_lowercase();
                            if stem_lower == pkg_name_lower || stem_lower.contains(&pkg_name_lower) {
                                return Some(path);
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Check %USERPROFILE%\scoop\apps\<pkg_name>\current\
    let scoop_base = std::env::var("SCOOP")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("USERPROFILE").unwrap_or_default();
            std::path::PathBuf::from(home).join("scoop")
        });

    let current_dir = scoop_base.join("apps").join(pkg_name).join("current");
    if current_dir.is_dir() {
        // 2a. Direct match: current/<pkg_name>.exe
        let direct_exe = current_dir.join(format!("{}.exe", pkg_name));
        if direct_exe.is_file() {
            return Some(direct_exe);
        }

        // 2b. Direct match in bin: current/bin/<pkg_name>.exe
        let bin_exe = current_dir.join("bin").join(format!("{}.exe", pkg_name));
        if bin_exe.is_file() {
            return Some(bin_exe);
        }

        // 2c. Any other .exe in current/ or current/bin/
        for dir in [&current_dir, &current_dir.join("bin")] {
            if dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map_or(false, |ext| ext.eq_ignore_ascii_case("exe"))
                        {
                            let file_name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or_default()
                                .to_lowercase();
                            if !file_name.contains("uninstall") && !file_name.contains("setup") {
                                return Some(path);
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Check Scoop shims: %USERPROFILE%\scoop\shims\<pkg_name>.exe
    let shims_dir = scoop_base.join("shims");
    if shims_dir.is_dir() {
        let shim_exe = shims_dir.join(format!("{}.exe", pkg_name));
        if shim_exe.is_file() {
            return Some(shim_exe);
        }
        let shim_cmd = shims_dir.join(format!("{}.cmd", pkg_name));
        if shim_cmd.is_file() {
            return Some(shim_cmd);
        }

        if let Ok(entries) = std::fs::read_dir(&shims_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem.to_lowercase().starts_with(&pkg_name_lower) {
                        return Some(path);
                    }
                }
            }
        }
    }

    None
}

#[cfg(windows)]
pub fn launch_installed_package(pkg: &ScoopPackage) -> anyhow::Result<()> {
    let target = find_installed_executable(&pkg.name);
    let target_str = match &target {
        Some(path) => path.to_string_lossy().to_string(),
        None => pkg.name.clone(),
    };

    println!("Launching '{}'...", target_str);

    let mut cmd = std::process::Command::new("cmd");
    cmd.args(["/c", "start", "", &target_str]);
    cmd.spawn()?;

    Ok(())
}

#[cfg(not(windows))]
pub fn launch_installed_package(pkg: &ScoopPackage) -> anyhow::Result<()> {
    println!("Launching '{}'...", pkg.name);
    std::process::Command::new(&pkg.name).spawn()?;
    Ok(())
}

fn run_scoop_install(pkg: &ScoopPackage, install_cmd: &str) -> anyhow::Result<InstallResult> {
    println!();
    println!("============================================================");
    println!(" ScoopFind: Installing {} (v{})", pkg.name, pkg.version);
    println!(" Bucket:    {}", pkg.bucket_name());
    println!(" Command:   {}", install_cmd);
    println!("============================================================");
    println!();

    #[cfg(windows)]
    let mut cmd = {
        let shell = detect_powershell_binary();
        let mut c = std::process::Command::new(shell);
        c.args(["-NoLogo", "-ExecutionPolicy", "Bypass", "-Command", install_cmd]);
        if shell == "powershell" {
            // Prevent PowerShell 7 environment pollution (PSModulePath) from breaking
            // Windows PowerShell 5.1 built-in cmdlets such as Get-FileHash.
            c.env_remove("PSModulePath");
        }
        c
    };

    #[cfg(not(windows))]
    let mut cmd = std::process::Command::new("sh");
    #[cfg(not(windows))]
    cmd.args(["-c", install_cmd]);

    cmd.stdin(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    let status = cmd.status()?;
    let exit_code = status.code().unwrap_or(-1);

    let action = prompt_post_install_action(&pkg.name, exit_code);

    Ok(InstallResult { exit_code, action })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_version() {
        assert_eq!(parse_args(["scoopfind", "--version"]), Ok(CliAction::Version));
        assert_eq!(parse_args(["scoopfind", "-v"]), Ok(CliAction::Version));
    }

    #[test]
    fn test_parse_args_help() {
        assert_eq!(parse_args(["scoopfind", "--help"]), Ok(CliAction::Help));
        assert_eq!(parse_args(["scoopfind", "-h"]), Ok(CliAction::Help));
    }

    #[test]
    fn test_parse_args_query() {
        assert_eq!(
            parse_args(["scoopfind", "git"]),
            Ok(CliAction::Run {
                initial_query: Some("git".to_string())
            })
        );
        assert_eq!(
            parse_args(["scoopfind", "ripgrep", "search"]),
            Ok(CliAction::Run {
                initial_query: Some("ripgrep search".to_string())
            })
        );
    }

    #[test]
    fn test_parse_args_empty() {
        assert_eq!(
            parse_args(["scoopfind"]),
            Ok(CliAction::Run { initial_query: None })
        );
    }

    #[test]
    fn test_parse_args_unknown_flag() {
        assert!(parse_args(["scoopfind", "--unknown"]).is_err());
    }

    #[test]
    fn test_detect_powershell_fallback() {
        assert_eq!(detect_powershell_from_path_env(None), "powershell");
        assert_eq!(
            detect_powershell_from_path_env(Some(std::ffi::OsString::from(""))),
            "powershell"
        );
    }

    #[test]
    fn test_detect_powershell_with_pwsh() {
        let temp_dir = std::env::temp_dir().join(format!(
            "scoopfind_test_pwsh_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&temp_dir);
        let fake_pwsh = temp_dir.join("pwsh.exe");
        let _ = std::fs::write(&fake_pwsh, b"");

        let result = detect_powershell_from_path_env(Some(temp_dir.clone().into_os_string()));
        let _ = std::fs::remove_file(&fake_pwsh);
        let _ = std::fs::remove_dir(&temp_dir);

        assert_eq!(result, "pwsh");
    }

    #[test]
    fn test_parse_post_install_choice() {
        // Success scenarios
        assert_eq!(parse_post_install_choice("1", true), PostInstallAction::LaunchAndQuit);
        assert_eq!(parse_post_install_choice("l", true), PostInstallAction::LaunchAndQuit);
        assert_eq!(parse_post_install_choice("L", true), PostInstallAction::LaunchAndQuit);
        assert_eq!(parse_post_install_choice("2", true), PostInstallAction::ReturnToApp);
        assert_eq!(parse_post_install_choice("r", true), PostInstallAction::ReturnToApp);
        assert_eq!(parse_post_install_choice("", true), PostInstallAction::ReturnToApp);
        assert_eq!(parse_post_install_choice("   ", true), PostInstallAction::ReturnToApp);
        assert_eq!(parse_post_install_choice("3", true), PostInstallAction::Quit);
        assert_eq!(parse_post_install_choice("q", true), PostInstallAction::Quit);
        assert_eq!(parse_post_install_choice("Q", true), PostInstallAction::Quit);

        // Failure scenarios
        assert_eq!(parse_post_install_choice("1", false), PostInstallAction::ReturnToApp);
        assert_eq!(parse_post_install_choice("", false), PostInstallAction::ReturnToApp);
        assert_eq!(parse_post_install_choice("2", false), PostInstallAction::Quit);
        assert_eq!(parse_post_install_choice("q", false), PostInstallAction::Quit);
    }

    #[test]
    fn test_find_installed_executable_fallback() {
        assert_eq!(find_installed_executable("non_existent_package_xyz_123"), None);
        if let Some(path) = find_installed_executable("deckcraft") {
            assert!(path.exists());
        }
    }
}

