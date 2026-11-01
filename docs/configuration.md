# Configuration & Usage Guide ⚙️

This guide covers CLI flags, keybindings, environment variables, and terminal compatibility for **`scoopfind`**.

---

## 📑 Table of Contents

1. [CLI Arguments & Flags](#-cli-arguments--flags)
2. [Complete Keybindings Reference](#-complete-keybindings-reference)
3. [Environment Variables](#-environment-variables)
4. [Terminal Compatibility & Themes](#-terminal-compatibility--themes)
5. [Troubleshooting & FAQ](#-troubleshooting--faq)

---

## 💻 CLI Arguments & Flags

Launch `scoopfind` directly from your command line:

```text
USAGE:
    scoopfind [OPTIONS] [QUERY]

ARGS:
    <QUERY>    Initial search query (e.g. 'scoopfind ripgrep')

OPTIONS:
    -h, --help       Print help information
    -v, --version    Print version information
```

### Examples

```powershell
# Launch interactive search empty
scoopfind

# Launch interactive search pre-loaded with query "git"
scoopfind git

# Launch interactive search with multiple words
scoopfind "visual studio code"
```

---

## ⌨️ Complete Keybindings Reference

`scoopfind` is designed for fast, keyboard-driven navigation with Vim-inspired shortcuts:

| Key | Context | Action |
| :--- | :--- | :--- |
| `<Enter>` | Search Bar | Execute search against Azure AI Search index |
| `<Enter>` | Results List / Details Pane | Open confirmation modal to install selected package |
| `<Esc>` | Any Pane | Return focus to the search bar |
| `<Esc>` (x2 within 500ms) | Search Bar | **Clear** the search input |
| `<Tab>` | Global | Cycle focus: **Search** ⇄ **Results List** ⇄ **Detail Pane** |
| `j` or `<↓>` | Results List | Select next package |
| `k` or `<↑>` | Results List | Select previous package |
| `g` or `<Home>` | Results List | Jump to the very first search result |
| `G` or `<End>` | Results List | Jump to the very last search result |
| `/` or `i` | Results List / Details | Jump immediately back to search input |
| `h` or `<←>` | Details Pane | Move focus back to the Results List |
| `<Ctrl+C>` (x2 within 1.5s) | Global | **Exit** the application safely |
| `/quit` or `/exit` + `<Enter>` | Search Bar | **Exit** the application from search bar |

### Confirmation Modal Keybindings

When prompted to install a package:

| Key | Action |
| :--- | :--- |
| `<Enter>` | Execute action of currently highlighted button |
| `y` or `Y` | Immediately confirm and start `scoop install` |
| `n` or `N` or `<Esc>` | Cancel and dismiss confirmation modal |
| `<Tab>`, `<←>`, `<→>`, `h`, `l` | Toggle between **[Install]** and **[Cancel]** |

---

## 🌐 Environment Variables

| Variable | Description | Default |
| :--- | :--- | :--- |
| `SCOOPFIND_API_KEY` | Override the Azure AI Search query key manually. Useful for testing or in private air-gapped environments. | Official public key (`DC6D2BBE65FC7313F2C52BBD2B0286ED`) |

### Setting a Custom Key

```powershell
# In PowerShell:
$env:SCOOPFIND_API_KEY = "YOUR_CUSTOM_AZURE_SEARCH_KEY_HERE"
scoopfind
```

---

## 🖥 Terminal Compatibility & Themes

`scoopfind` is built on top of [Ratatui](https://github.com/ratatui/ratatui) and [Crossterm](https://github.com/crossterm-rs/crossterm).

### Recommended Terminals
For the best visual experience, UTF-8 rendering, and true color support:
- **Windows Terminal** (Recommended)
- **WezTerm**
- **Alacritty**
- **VS Code Integrated Terminal**

### Legacy Terminals (`conhost.exe`)
Classic Windows Command Prompt (`cmd.exe`) and legacy PowerShell consoles are supported, but standard ANSI 16-color fallbacks will apply.

---

## ❓ Troubleshooting & FAQ

### 1. Terminal screen garbled after sudden termination
If `scoopfind` crashes or your terminal window closes abruptly, your terminal might remain in raw mode.
- **PowerShell / CMD fix:**
  ```powershell
  # Reset console mode in PowerShell
  [Console]::CursorVisible = $true
  ```
- Or simply close and open a fresh terminal tab.

### 2. Double `<Ctrl+C>` does not exit immediately
To prevent accidental terminations while typing search queries, `scoopfind` requires two `<Ctrl+C>` presses within 1.5 seconds. Alternatively, you can type `/quit` or `/exit` and press `<Enter>`.
