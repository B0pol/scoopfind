# scoopfind 🔎

A blazing-fast, modern Terminal User Interface (TUI) package search and pager for [Scoop](https://scoop.sh) on Windows, powered directly by the official Scoop Azure AI Search API.

---

## ✨ Features

- **Fast Real-Time Search:** Search across all Scoop buckets instantly with zero local cloning, heavy downloads, or manual index maintenance.
- **Split-Screen Package Inspection:**
  - **Results List:** Compact overview displaying package name, version, bucket badge, repository stars, and relative update recency (e.g. `now`, `3h`, `12d`, `2mo`, `1y`).
  - **Package Details Pane:** Instant inspection of package metadata including description, bucket repository, homepage URL, SPDX license, git commit SHA, and exact update timestamp.
  - **Ready-to-Run Install Command:** Automatically generates the correct `scoop bucket add <bucket>` and `scoop install <app>` commands.
- **Direct Interactive Installation:** Press `<Enter>` on any result to open a confirmation prompt and install packages directly without leaving the app.
- **Vim-Friendly & Intuitive Navigation:** Fluid browsing with arrow keys or vim navigation (`j` / `k`, `g` / `G`), instant search filtering, and seamless pane focus cycling with `<Tab>`.
- **Safe Exit Safeguards:** Protection against accidental terminations—requires double `<Ctrl+C>` or typing `/quit` / `/exit`, with double `<Esc>` to quickly clear the search query.
- **Zero Setup & Resilient Connectivity:** Ready to use out-of-the-box with automatic self-healing API connectivity, ensuring queries remain reliable even if upstream service keys rotate.

---

## ⌨️ Keybindings

| Key | Action |
| :--- | :--- |
| `<Enter>` | Execute search (Search Bar) / Prompt install (Results & Details) |
| `<Esc>` | Return focus to the search bar |
| `<Esc>` (x2 within 500ms) | **Clear** the search bar query |
| `<Tab>` | Cycle focus: **Search Bar** ⇄ **Results List** ⇄ **Details Pane** |
| `<↑>` / `<↓>` (or `k` / `j`) | Navigate up/down through search results |
| `<Home>` / `<End>` (or `g` / `G`) | Jump to top / bottom of search results |
| `<Ctrl+C>` (x2 within 1.5s) | **Exit** the application |
| `/quit` or `/exit` + `<Enter>` | **Exit** the application from search bar |

*(Note: The `q` key does not exit the app, preventing accidental terminations).*

---

## 🚀 Installation

### Option 1: Via Scoop (Recommended)

Install directly using the manifest:

```powershell
scoop install https://raw.githubusercontent.com/B0pol/scoopfind/master/scoop/scoopfind.json
```

*(Or if added to a custom bucket or Scoop Extras once accepted: `scoop install scoopfind`)*

### Option 2: Pre-compiled Binary (GitHub Releases)

1. Download the latest `scoopfind-*-windows-x64.zip` from [GitHub Releases](https://github.com/B0pol/scoopfind/releases).
2. Extract `scoopfind.exe` and place it in your `PATH` (e.g., `~/scoop/shims` or `C:\Windows\System32`).

### Option 3: Via Cargo

```powershell
cargo install scoopfind
```

### Option 4: Build from Source

#### Prerequisites
- [Rust Toolchain](https://rustup.rs) (Rust 2024 / 1.99+)
- C Linker on Windows:
  - **Option A (Lightweight via Scoop):** `scoop install mingw`
  - **Option B (Microsoft MSVC):** Visual Studio C++ Build Tools (`link.exe`)

```powershell
# Clone the repository
git clone https://github.com/B0pol/scoopfind.git
cd scoopfind

# Run the test suite
cargo test

# Build the release binary
cargo build --release

# The binary will be available at:
# target/release/scoopfind.exe
```

---

## 🏗️ Architecture

All application state is centralized in an owned `App` struct to maintain simple, idiomatic Rust code free of complex lifetime annotations:

```
scoopfind/
├── src/
│   ├── main.rs     # Terminal lifecycle (crossterm raw mode) & async Tokio event loop
│   ├── app.rs      # Centralized App state, cursor handling, navigation, and exit safeguards
│   ├── api.rs      # Azure AI Search client & dynamic fallback key recovery from scoop.sh
│   ├── models.rs   # Deserialization models, bucket resolver, and relative age formatters
│   └── ui.rs       # Ratatui split-pane layout, widgets, and dynamic responsive footer
├── docs/           # Architecture, contributing, configuration, and releasing guides
├── scoop/
│   └── scoopfind.json # Scoop package manifest template
├── .cargo/
│   └── config.toml # Portable target configuration (x86_64-pc-windows-gnu)
├── .github/
│   └── workflows/  # CI testing and automated GitHub release pipelines
└── Cargo.toml      # Package metadata & dependency specifications
```

---

## 📚 Documentation

For in-depth guides and contributor resources, visit the [docs](docs/README.md) directory:

- 🏛️ **[Architecture & Design](docs/architecture.md)** — Deep dive into the event loop, state machine, and self-healing search engine.
- ⚙️ **[Configuration & Usage](docs/configuration.md)** — CLI arguments, keybindings reference, environment variables, and terminal support.
- 🤝 **[Contributing Guide](docs/contributing.md)** — Toolchain requirements, dev setup, code style, and Conventional Commits.
- 🚀 **[Releasing Guide](docs/releasing.md)** — Step-by-step maintainer guide for version bumping and publishing.
```

---

## 🧪 Testing

Run the automated test suite verifying live search queries, dynamic key recovery, and navigation logic:

```bash
cargo test
```

---

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.
