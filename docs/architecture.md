# Architecture & Technical Design 🏛️

This document details the internal architecture, design principles, and component interactions of **`scoopfind`**.

---

## 📑 Table of Contents

1. [Design Philosophy](#-design-philosophy)
2. [Component Architecture](#-component-architecture)
3. [Centralized State Machine (`App`)](#-centralized-state-machine-app)
4. [Non-Blocking Async Search Engine](#-non-blocking-async-search-engine)
5. [Azure AI Search Client & Self-Healing](#-azure-ai-search-client--self-healing)
6. [TUI Rendering Pipeline (`Ratatui`)](#-tui-rendering-pipeline-ratatui)
7. [Direct Package Installation Subprocess](#-direct-package-installation-subprocess)
8. [Module Map & Responsibilities](#-module-map--responsibilities)

---

## 💡 Design Philosophy

- **100% Online & Zero Indexing:** No local git bucket cloning or sqlite databases. All search requests query the official Scoop Azure AI Search API directly.
- **Simplicity & Maintainability:** Favor owned data (`String`, `Vec<T>`) and `.clone()` over complex lifetime annotations (`'a`). All state is held in a single `App` struct.
- **Non-Blocking User Experience:** The terminal event loop renders at a steady ~30ms poll interval. Network requests and key-scraping fallbacks run in an asynchronous Tokio background task.
- **Terminal Safety:** Double-exit safeguards and automated panic hooks guarantee that raw terminal mode is always restored, even upon abnormal termination.

---

## 🏗 Component Architecture

```
┌────────────────────────────────────────────────────────┐
│                   Terminal Event Loop                  │
│                        (main.rs)                       │
│  - Crossterm raw mode & alternate screen lifecycle     │
│  - Polls keyboard input events (~30ms timeout)         │
│  - Dispatches actions to App state machine             │
└───────────┬────────────────────────────────┬───────────┘
            │                                │
            ▼ (Key events)                   ▼ (Frame tick)
┌────────────────────────┐      ┌────────────────────────┐
│    App State Machine   │      │    UI Renderer         │
│        (app.rs)        │◄────►│        (ui.rs)         │
│ - Search query buffer  │      │ - Ratatui split-pane   │
│ - Focus cycle          │      │ - Results list widget  │
│ - Selection indices    │      │ - Detail viewer widget │
│ - Confirmation modal   │      │ - Dynamic status footer│
└───────────┬────────────┘      └────────────────────────┘
            │
            │ (WorkerRequest: Search)
            ▼
┌────────────────────────────────────────────────────────┐
│                Tokio Async Background Worker           │
│                        (api.rs)                        │
│ - Listens on tokio::sync::mpsc channel                 │
│ - Builds & executes HTTP queries to Azure AI Search    │
│ - Self-healing: scrapes scoop.sh frontend on 401/403   │
│ - Sends WorkerResponse back to main event loop         │
└────────────────────────────────────────────────────────┘
```

---

## 🧠 Centralized State Machine (`App`)

The entire interactive state of the application is encapsulated in `src/app.rs`:

```rust
pub struct App {
    pub query: String,
    pub cursor_position: usize,
    pub focus: Focus,
    pub results: Vec<ScoopPackage>,
    pub selected_index: usize,
    pub is_loading: bool,
    pub status_message: String,
    pub confirming_install: Option<ScoopPackage>,
    pub confirm_button: ConfirmButton,
    pub pending_install: Option<ScoopPackage>,
    pub should_quit: bool,
    // ... exit guards and escape timers
}
```

### Focus Cycle & Navigation
The UI maintains three distinct focus areas:
1. `Focus::SearchInput`: Focused on the search text box. Typing edits the query, `<Enter>` initiates a search.
2. `Focus::ResultsList`: Browses search results using arrow keys or vim navigation (`j` / `k`, `g` / `G`).
3. `Focus::DetailPane`: Inspects full manifest details, homepage URL, license, and git commit SHA.

Pressing `<Tab>` seamlessly cycles between these three areas.

### Safety Safeguards
- **Accidental Exit Protection:** Pressing `<Ctrl+C>` requires a second press within 1.5 seconds to exit. Alternatively, typing `/quit` or `/exit` in the search bar exits gracefully.
- **Fast Search Clearing:** Pressing `<Esc>` twice within 500ms clears the search bar and returns focus.

---

## ⚡ Non-Blocking Async Search Engine

To ensure keystrokes, scrolling, and modal interactions never stutter during network operations:

1. **Request Channel:** The main UI thread issues searches via an unbounded Tokio channel:
   ```rust
   let (req_tx, mut req_rx) = mpsc::unbounded_channel::<WorkerRequest>();
   ```
2. **Response Channel:** The background worker processes the query and returns results:
   ```rust
   let (resp_tx, mut resp_rx) = mpsc::unbounded_channel::<WorkerResponse>();
   ```
3. **Out-of-Order Mitigation:** Responses are validated against `app.query.trim()`. Outdated responses from older keystrokes or cancelled searches are automatically discarded.

---

## 🔍 Azure AI Search Client & Self-Healing

The search client in `src/api.rs` queries the official Azure AI Search endpoint used by `scoop.sh`:

- **Endpoint:** `https://scoopsearch.search.windows.net/indexes/apps/docs/search?api-version=2020-06-30`
- **Default Query Key:** `DC6D2BBE65FC7313F2C52BBD2B0286ED` (official public key).

### Dynamic Key Recovery Pipeline
If Azure ever rotates the API query key:
```
HTTP POST to Azure AI Search
         │
         ├──► [200 OK] ──► Parse & Return Packages
         │
         └──► [401 / 403 Forbidden]
                   │
                   ▼
       Fetch https://scoop.sh HTML
                   │
                   ▼
       Extract Vite Bundle (/assets/index-*.js)
                   │
                   ▼
       Regex Search: [A-F0-9]{32}
                   │
                   ▼
       Cache New Key to %TEMP%/scoopfind_cached_key.txt
                   │
                   ▼
       Retry Original Search Query
```

Maintainers and users can also override the API key at any time using the `SCOOPFIND_API_KEY` environment variable.

---

## 🎨 TUI Rendering Pipeline (`Ratatui`)

The rendering engine (`src/ui.rs`) uses a responsive split-pane layout:

1. **Root Layout:** Divided vertically into:
   - **Header / Search Box** (3 lines)
   - **Main Body** (fills available height)
   - **Status Bar / Footer** (1 line)
2. **Main Body Split:** Divided horizontally into:
   - **Results List Pane** (35% width, minimum 30 columns)
   - **Package Detail Pane** (65% width)
3. **Confirmation Modal:** Rendered as a centered floating popup over a darkened backdrop whenever an install action is requested.

---

## 📦 Direct Package Installation Subprocess

When a package installation is confirmed:

1. **Terminal Teardown:** The TUI safely disables Crossterm raw mode and leaves the alternate screen, restoring the user's standard console.
2. **Process Execution:** Launches PowerShell with `-ExecutionPolicy Bypass`:
   ```powershell
   scoop bucket add <bucket> <repo_url>; scoop install <package>
   ```
3. **Interactive Console:** Standard input/output/error streams are inherited, allowing Scoop to display its real-time download progress and prompts.
4. **Terminal Restoration:** Once the process completes and the user presses `<Enter>`, raw mode and alternate screen are re-enabled, redrawing the UI in place.

---

## 📂 Module Map & Responsibilities

| File | Purpose |
| :--- | :--- |
| [`src/main.rs`](../src/main.rs) | CLI argument parsing, terminal setup/cleanup, async Tokio worker lifecycle, and main event loop. |
| [`src/app.rs`](../src/app.rs) | Centralized state machine, focus cycling, cursor tracking, and navigation safety safeguards. |
| [`src/api.rs`](../src/api.rs) | Azure AI Search client, request construction, response parsing, and dynamic HTML scraping key recovery. |
| [`src/models.rs`](../src/models.rs) | Data deserialization structures (`ScoopPackage`, `AzureSearchResponse`), bucket resolver, and relative age formatters. |
| [`src/ui.rs`](../src/ui.rs) | Ratatui widgets, split-pane layout geometry, color themes, modal popups, and responsive footer formatting. |
