# Contributing to scoopfind 🤝

Thank you for your interest in contributing to **`scoopfind`**! This document provides guidelines and instructions for setting up your development environment, adhering to code style, and submitting contributions.

---

## 📑 Table of Contents

1. [Code of Conduct](#-code-of-conduct)
2. [Prerequisites & Development Environment](#-prerequisites--development-environment)
3. [Building & Running Locally](#-building--running-locally)
4. [Testing & Quality Assurance](#-testing--quality-assurance)
5. [Code Style & Architectural Constraints](#-code-style--architectural-constraints)
6. [Conventional Commits Specification](#-conventional-commits-specification)
7. [Submitting a Pull Request](#-submitting-a-pull-request)

---

## 📜 Code of Conduct

Be respectful, welcoming, and constructive. We are committed to providing a friendly and inclusive environment for everyone.

---

## 🛠 Prerequisites & Development Environment

`scoopfind` requires:
- **Rust Toolchain:** Rust 2024 edition (Rust 1.99 or higher). Install via [rustup.rs](https://rustup.rs).
- **C Linker on Windows:**
  - **Option A (Lightweight via Scoop):** `scoop install mingw` (configures `x86_64-pc-windows-gnu`).
  - **Option B (Microsoft Visual C++):** Visual Studio C++ Build Tools (`link.exe`).

> [!WARNING]
> If using MinGW via Scoop, do not use `tdm-gcc` (it lacks `libgcc_eh.a`). Never commit machine-specific absolute paths to `.cargo/config.toml`.

---

## 🚀 Building & Running Locally

```powershell
# 1. Clone the repository
git clone https://github.com/B0pol/scoopfind.git
cd scoopfind

# 2. Build debug binary
cargo build

# 3. Run scoopfind locally
cargo run

# 4. Or launch with an initial search query
cargo run -- ripgrep
```

---

## 🧪 Testing & Quality Assurance

Always run the automated test suite before opening a PR:

```powershell
# Run all unit and integration tests
cargo test

# Run tests with stdout output visible
cargo test -- --nocapture

# Check compilation without producing artifacts
cargo check
```

### Test Coverage Highlights
- `tests::test_azure_search_real_api`: Live search against the Azure AI Search index.
- `tests::test_auto_heal_with_bad_key`: Live self-healing key recovery from `scoop.sh`.
- `tests::test_relative_age_short` & `tests::test_relative_age_detailed`: Relative time formatters.
- `tests::test_parse_args_*`: CLI argument parsing flags and initial query handling.

---

## 🎨 Code Style & Architectural Constraints

When submitting code changes, follow these core repository guidelines:

1. **Favor Simplicity over Complexity:**
   - Keep lifetimes simple. Favor owned data types (`String`, `Vec<T>`) and `.clone()` over intricate reference lifetimes (`'a`).
   - Centralize all application state within the `App` struct in `src/app.rs`.
2. **Online-Only Architecture:**
   - `scoopfind` is intentionally an online-first search tool. Do not add local git bucket cloning or offline SQLite indexers.
3. **Preserve Documentation & Comments:**
   - Maintain documentation integrity. Keep existing code comments and docstrings intact unless directly modifying the underlying logic.

---

## 📝 Conventional Commits Specification

All Git commits in this repository must strictly adhere to the [Conventional Commits v1.0.0](https://www.conventionalcommits.org/) specification.

### Commit Format
```text
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Allowed Types

| Type | Purpose |
| :--- | :--- |
| `feat` | A new feature |
| `fix` | A bug fix |
| `docs` | Documentation changes only |
| `style` | Formatting, whitespace, or visual style adjustments |
| `refactor` | Code restructuring without adding features or fixing bugs |
| `perf` | Code changes that improve performance |
| `test` | Adding, updating, or correcting tests |
| `build` | Changes to build configuration or external dependencies |
| `ci` | Changes to CI/CD workflows and deployment scripts |
| `chore` | Maintenance tasks and tooling updates |

### Rules & Examples
- Use imperative, present tense: `feat(search): add fuzzy query highlighting`.
- Lowercase description with **no trailing period**.
- Valid examples:
  ```text
  feat(install): add confirm modal before running scoop
  fix(api): handle trailing whitespace in api key override
  docs: update architecture overview in docs/architecture.md
  ```

---

## 📤 Submitting a Pull Request

1. **Fork the repository** on GitHub.
2. **Create a topic branch:**
   ```powershell
   git checkout -b feat/my-awesome-feature
   ```
3. **Make your changes** following the code style and guidelines.
4. **Run pre-flight checks:**
   ```powershell
   cargo test
   cargo check
   ```
5. **Commit your changes** with conventional commit messages.
6. **Push to your fork** and submit a Pull Request to `master`.
