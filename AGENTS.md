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
