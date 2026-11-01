# AGENTS.md

Technical constraints, toolchain learnings, and gotchas for AI agents working on `scoopfind`.

---

## 1. Architecture Constraints

- **100% Online Search:** Queries the official Azure AI Search API directly. Do not implement local git bucket cloning or offline manifest indexing.
- **Keep Rust Simple:** Favor owned data (`String`, `Vec<T>`) and `.clone()` over complex lifetimes (`'a`). Keep all UI/search state centralized in `App` (`src/app.rs`).
- **Dynamic Key Self-Healing:** If the search API returns 401/403, `src/api.rs` scrapes `scoop.sh` to extract the updated key from the Vite bundle and retries.

---

## 2. Windows Toolchain & Build Gotchas

- **MinGW GCC Linker:** Configured for `x86_64-pc-windows-gnu` using standard Scoop `mingw`. Do NOT use `tdm-gcc` (it lacks `libgcc_eh.a`). Never put absolute user paths in `.cargo/config.toml`.
- **Reqwest TLS:** Uses native Windows SChannel (`schannel`). Avoid `rustls` with `aws-lc-sys`, which causes `undefined reference to nanosleep64` linker errors under MinGW.
- **Locked Executable (Error 5):** If `scoopfind.exe` is running in another terminal, `cargo build` will fail with OS error 5 (`Accès refusé`). Terminate the running instance before rebuilding.

---

## 3. Verification

- Run `cargo test` before submitting changes (covers API queries, key recovery, navigation, and relative time formatting).

---

## 4. Release & Publishing Workflow

Follow this step-by-step workflow when cutting and publishing a new release:

### 1. Version Bumping
- When preparing a release, bump the `version` field in `Cargo.toml`.
- In `scoop/scoopfind.json`, update the `version` and download URL (`architecture.64bit.url`).
- Clear the `"hash"` field (`""`) in `scoop/scoopfind.json` (it will be filled after the release archive is built).

### 2. Verification Before Release
- **Unit and Integration Tests:** Run `cargo test`.
- **Packaging Dry-Run:** Run `cargo publish --dry-run` to ensure packaging rules, file exclusions (`.cargo`, `AGENTS.md`, `.github`, `scoop`, `docs`), and crates.io publishing requirements pass without errors. (See `docs/releasing.md` for the complete guide).

### 3. Git Tagging & Release Automation
- Commit changes using conventional commits:
  ```bash
  git add Cargo.toml Cargo.lock scoop/scoopfind.json
  git commit -m "chore(release): bump version to vX.Y.Z"
  ```
- Create and push a git tag `vX.Y.Z`:
  ```bash
  git tag vX.Y.Z
  git push origin master && git push origin vX.Y.Z
  ```
- **Automated GitHub Release:** Pushing the `v*` tag triggers the GitHub Actions workflow in `.github/workflows/release.yml`:
  - Builds the optimized release binary targeting `x86_64-pc-windows-msvc`.
  - Packages `scoopfind.exe`, `README.md`, and `LICENSE` into `scoopfind-vX.Y.Z-windows-x64.zip`.
  - Computes the SHA256 checksum and outputs it to `scoopfind-vX.Y.Z-windows-x64.zip.sha256`.
  - Automatically creates a GitHub Release and publishes the zip archive and `.sha256` checksum asset.

### 4. Scoop Manifest Update Post-Release
- Retrieve the SHA256 hash from the generated release asset (`scoopfind-vX.Y.Z-windows-x64.zip.sha256`) on GitHub Releases.
- Update the `"hash"` field in `scoop/scoopfind.json` with this SHA256 string.
- Commit the updated manifest (`chore(scoop): update scoopfind manifest hash for vX.Y.Z`) and push or submit a PR to the target Scoop bucket repository.

### 5. Crates.io Publishing
- Once the git release is tagged and pushed:
  ```bash
  cargo publish
  ```
