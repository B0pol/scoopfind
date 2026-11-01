# Releasing scoopfind 🚀

This guide provides maintainers with a comprehensive, step-by-step walkthrough for cutting, verifying, and publishing a new release of **`scoopfind`**.

---

## 📑 Table of Contents

1. [Overview & Release Architecture](#-overview--release-architecture)
2. [Prerequisites](#-prerequisites)
3. [Step-by-Step Release Checklist](#-step-by-step-release-checklist)
   - [Step 1: Version Bumping](#step-1-version-bumping)
   - [Step 2: Pre-flight Verification](#step-2-pre-flight-verification)
   - [Step 3: Git Commit & Tagging](#step-3-git-commit--tagging)
   - [Step 4: GitHub Actions Release Automation](#step-4-github-actions-release-automation)
   - [Step 5: Updating Scoop Manifest Hash](#step-5-updating-scoop-manifest-hash)
   - [Step 6: Publishing to Crates.io](#step-6-publishing-to-cratesio)
4. [Troubleshooting & Gotchas](#-troubleshooting--gotchas)
5. [Quick Release Cheat Sheet](#-quick-release-cheat-sheet)

---

## 🏗 Overview & Release Architecture

Every release of `scoopfind` distributes assets across three distribution channels:

```
                  ┌───────────────────────────────┐
                  │   Tag & Push: vX.Y.Z          │
                  └──────────────┬────────────────┘
                                 │
         ┌───────────────────────┼────────────────────────┐
         ▼                       ▼                        ▼
┌──────────────────┐   ┌──────────────────┐   ┌──────────────────────┐
│ GitHub Actions   │   │ Scoop Manifest   │   │ Crates.io            │
│ (release.yml)    │   │ (scoopfind.json) │   │ (cargo publish)      │
├──────────────────┤   ├──────────────────┤   ├──────────────────────┤
│ Builds MSVC x64  │   │ Updated with     │   │ Distributes source & │
│ Packages ZIP:    │   │ version, URL,    │   │ metadata for         │
│  - scoopfind.exe │   │ and SHA-256 hash │   │ cargo install        │
│  - README.md     │   │ from GitHub      │   │                      │
│  - LICENSE       │   │ Release archive  │   │                      │
│ Computes .sha256 │   │                  │   │                      │
│ Publishes Assets │   │                  │   │                      │
└──────────────────┘   └──────────────────┘   └──────────────────────┘
```

### Published Assets Summary

| Channel | Target Audience | Published Artifacts |
| :--- | :--- | :--- |
| **GitHub Releases** | Windows users (portable manual install) | `scoopfind-vX.Y.Z-windows-x64.zip`<br>`scoopfind-vX.Y.Z-windows-x64.zip.sha256` |
| **Scoop Bucket** | Windows users (`scoop install`) | `scoop/scoopfind.json` updated with download URL & SHA-256 |
| **Crates.io** | Rust developers (`cargo install scoopfind`) | Crate tarball published via `cargo publish` |

---

## 📋 Prerequisites

Before cutting a release, make sure you have:

- [x] Push permissions on the [B0pol/scoopfind](https://github.com/B0pol/scoopfind) repository.
- [x] Rust toolchain installed (`cargo --version`, `rustc --version`).
- [x] An active token configured for crates.io:
  ```powershell
  cargo login
  ```
- [x] (Optional but recommended) GitHub CLI (`gh`) for monitoring CI and fetching release assets.

---

## 🚶 Step-by-Step Release Checklist

### Step 1: Version Bumping

Set your target release version (e.g. `X.Y.Z = 0.1.1`):

#### 1. Update `Cargo.toml`
Update the `version` field at the top of `Cargo.toml`:
```toml
[package]
name = "scoopfind"
version = "X.Y.Z"
```

#### 2. Update `scoop/scoopfind.json`
Update the `version` and the `architecture.64bit.url` fields, and **clear the `hash` field** to an empty string `""` (the hash will be filled in Step 5 after CI generates the archive):
```json
{
    "version": "X.Y.Z",
    "description": "A blazing-fast, modern TUI package search and pager for Scoop on Windows",
    "homepage": "https://github.com/B0pol/scoopfind",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "https://github.com/B0pol/scoopfind/releases/download/vX.Y.Z/scoopfind-vX.Y.Z-windows-x64.zip",
            "hash": ""
        }
    },
    "bin": "scoopfind.exe",
    "checkver": "github",
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "https://github.com/B0pol/scoopfind/releases/download/v$version/scoopfind-v$version-windows-x64.zip"
            }
        }
    }
}
```

---

### Step 2: Pre-flight Verification

Run all tests and simulate package publishing locally:

```powershell
# 1. Run full test suite (API queries, key fallback, models, navigation)
cargo test

# 2. Update Cargo.lock with the new version and check compilation
cargo check

# 3. Verify crate packaging rules, exclusions, and crates.io readiness
cargo publish --dry-run
```

> [!TIP]
> Inspect the list of files included in the crate package to ensure no extraneous local files or caches are bundled:
> ```powershell
> cargo package --list
> ```
> Unwanted files and directories like `.cargo`, `.github`, `scoop`, `AGENTS.md`, and `docs` are excluded via `Cargo.toml` (`package.exclude`).

---

### Step 3: Git Commit & Tagging

Commit the version bump using a **Conventional Commit** message and create an annotated tag:

```powershell
# Define version variable (e.g., 0.1.1)
$VERSION = "0.1.1"

# Stage modified files
git add Cargo.toml Cargo.lock scoop/scoopfind.json

# Commit with conventional commits specification
git commit -m "chore(release): bump version to v$VERSION"

# Create git tag
git tag -a "v$VERSION" -m "Release v$VERSION"

# Push commit to master and push the new tag
git push origin master
git push origin "v$VERSION"
```

---

### Step 4: GitHub Actions Release Automation

Pushing the `v*` tag triggers the automated release pipeline in [`.github/workflows/release.yml`](../.github/workflows/release.yml):

1. **Build:** Runs on `windows-latest` with target `x86_64-pc-windows-msvc`.
2. **Packaging:**
   - Creates `scoopfind-vX.Y.Z-windows-x64/`
   - Copies `scoopfind.exe`, `README.md`, and `LICENSE`
   - Compresses into `scoopfind-vX.Y.Z-windows-x64.zip`
   - Computes SHA-256 hash into `scoopfind-vX.Y.Z-windows-x64.zip.sha256`
3. **Publishing:** Publishes a GitHub Release with auto-generated release notes and attaches the zip archive and `.sha256` asset.

#### Monitor the Release Workflow

You can monitor workflow execution using GitHub CLI or your browser:

```powershell
# Watch the running workflow in terminal
gh run watch

# Or view release status directly
gh release view "v$VERSION"
```

---

### Step 5: Updating Scoop Manifest Hash

Once the GitHub Actions workflow completes, retrieve the SHA-256 hash and update the Scoop manifest:

#### 1. Retrieve the SHA-256 Hash

**Option A (Using PowerShell):**
```powershell
$VERSION = "0.1.1"
$shaUrl = "https://github.com/B0pol/scoopfind/releases/download/v$VERSION/scoopfind-v$VERSION-windows-x64.zip.sha256"
$shaContent = (Invoke-RestMethod -Uri $shaUrl).Trim()
$hash = ($shaContent -split '\s+')[0]
Write-Host "Release SHA256: $hash"
```

**Option B (Using GitHub CLI):**
```powershell
$VERSION = "0.1.1"
gh release download "v$VERSION" -p "*.sha256" -O -
```

#### 2. Update `scoop/scoopfind.json`
Paste the SHA-256 checksum string into the `"hash"` field in `scoop/scoopfind.json`:
```json
    "architecture": {
        "64bit": {
            "url": "https://github.com/B0pol/scoopfind/releases/download/v0.1.1/scoopfind-v0.1.1-windows-x64.zip",
            "hash": "c1f7b8..."
        }
    }
```

#### 3. Commit and Push Manifest Update
```powershell
git add scoop/scoopfind.json
git commit -m "chore(scoop): update scoopfind manifest hash for v$VERSION"
git push origin master
```

> [!NOTE]
> If maintaining an external or community Scoop bucket (e.g. `ScoopInstaller/Main` or a custom bucket repo), copy this updated `scoopfind.json` manifest into that bucket repository and submit a commit or Pull Request.

---

### Step 6: Publishing to Crates.io

Now that the release tag and binary assets are live on GitHub, publish the crate to [crates.io](https://crates.io/crates/scoopfind):

```powershell
# Publish to crates.io
cargo publish
```

Verify the new release is listed at:
👉 **`https://crates.io/crates/scoopfind`**

---

## 🛠 Troubleshooting & Gotchas

### 1. Error: OS Error 5 (`Accès refusé` / Access Denied)
- **Symptom:** `cargo build` or `cargo test` fails when linking with `OS error 5: Accès refusé` while trying to write `target/.../scoopfind.exe`.
- **Cause:** An instance of `scoopfind.exe` is currently running in another terminal or background process, locking the binary file on Windows.
- **Solution:** Terminate running instances before rebuilding:
  ```powershell
  # In PowerShell:
  Get-Process -Name scoopfind -ErrorAction SilentlyContinue | Stop-Process -Force

  # Or using CMD:
  taskkill /F /IM scoopfind.exe
  ```

### 2. MinGW (Local) vs MSVC (CI) Toolchains
- Local development is configured for `x86_64-pc-windows-gnu` via `.cargo/config.toml` using standard MinGW GCC.
- GitHub Actions CI builds release archives with `x86_64-pc-windows-msvc` for maximum compatibility on clean Windows installations without MinGW runtime dependencies.
- Do not check in absolute paths in `.cargo/config.toml`.

### 3. Untracked Files Bundled in Crates.io Crate
- If extra files accidentally get bundled into the package, verify the `exclude` list in `Cargo.toml`:
  ```toml
  exclude = [".cargo", "AGENTS.md", ".github", "scoop", "docs"]
  ```
- Always preview package contents with `cargo package --list` before running `cargo publish`.

### 4. Failed Workflow or Incorrect Tag
- If CI fails due to an infrastructure glitch, re-run the failed job directly in GitHub Actions.
- If you need to re-tag before publishing to Crates.io:
  ```powershell
  # Delete local tag
  git tag -d vX.Y.Z
  # Delete remote tag
  git push origin :refs/tags/vX.Y.Z
  ```
  *(Note: Crates.io versions are permanently immutable once published via `cargo publish`).*

---

## ⚡ Quick Release Cheat Sheet

```powershell
# 1. Update version in Cargo.toml & scoop/scoopfind.json (hash: "")
# 2. Pre-flight checks
cargo test
cargo publish --dry-run

# 3. Commit, tag, and push
$V = "0.1.1"
git add Cargo.toml Cargo.lock scoop/scoopfind.json
git commit -m "chore(release): bump version to v$V"
git tag -a "v$V" -m "Release v$V"
git push origin master
git push origin "v$V"

# 4. Wait for GitHub Actions release build (gh run watch)
# 5. Get sha256 and update scoop/scoopfind.json hash
$hash = ((Invoke-RestMethod "https://github.com/B0pol/scoopfind/releases/download/v$V/scoopfind-v$V-windows-x64.zip.sha256").Trim() -split '\s+')[0]
Write-Host "Hash: $hash"
# (Paste $hash into scoop/scoopfind.json)
git add scoop/scoopfind.json
git commit -m "chore(scoop): update scoopfind manifest hash for v$V"
git push origin master

# 6. Publish to Crates.io
cargo publish
```
