# Hacker News Multi-platform Monorepo

This repository hosts a pnpm + Turbo + Cargo monorepo that targets the web and the terminal using one shared Rust implementation that can be compiled to WebAssembly. The main goal is to display the top Hacker News stories inside a Vite React SPA and a Rust TUI built with [ratatui](https://github.com/ratatui/ratatui). Both surfaces consume the same business logic:

```
┌────────────┐   wasm-bindgen   ┌──────────────┐
│  hn-core   │ ───────────────▶ │  hn-wasm     │ ──► Vite SPA (apps/web)
│ (Rust API) │                  │ (WASM crate) │
└────────────┘                  └──────────────┘
      │                                   ▲
      │  native dependency                │ npm package @hn/wasm
      ▼                                   │
┌────────────┐                             │
│  hn-tui    │ ◄───────────────────────────┘
│ (ratatui)  │  consumes hn-core directly, mirrors WASM logic
└────────────┘
```

Highlights:
- Shared Rust client (`hn-core`) fetches Hacker News top stories and comments, automatically switching between `reqwest` (native) and `gloo-net` (wasm) transports.
- `hn-wasm` wraps the shared logic with `wasm-bindgen`/`serde-wasm-bindgen` and is published to the JS workspace as `@hn/wasm`.
- The SPA renders story lists via Vite + React and lazily loads the WASM module.
- The TUI renders a split-pane interface: left pane lists stories with arrow/`j`/`k` navigation; right pane streams comments for the selected story in near real time.

## Structure

```
apps/
  web/   # Vite + React SPA
  tui/   # Ratatui-powered terminal UI
crates/
  hn-core/ # Shared Rust logic (fetch + transform)
  hn-wasm/ # wasm-bindgen wrapper exposed to JS tooling
packages/
  hn-wasm/ # npm-friendly wrapper around the wasm artifact
```

## Getting Started

### 1. Install toolchains

- Node.js 20+ and pnpm (`corepack enable` is recommended)
- Rust stable (see `rust-toolchain.toml`) plus the `wasm32-unknown-unknown` target
- `wasm-pack` for building the WebAssembly package

### 2. Install dependencies

```sh
pnpm install
```

### 3. Core commands

| Command | Description |
| --- | --- |
| `pnpm wasm:build` | One-off build of `hn-wasm` via `wasm-pack`; writes to `packages/hn-wasm/pkg`. Required before the first SPA run or CI build. |
| `pnpm dev` | Runs Turbo tasks for `apps/web` and `@hn/wasm`. You get the Vite dev server plus a Rust watcher (`chokidar` + `wasm-pack`) so editing any crate rebuilds the WASM artifacts automatically. |
| `pnpm tui` | Launches the terminal UI (`cargo run -p hn-tui`). Keep this in a separate terminal since it needs a dedicated TTY. |
| `pnpm build` / `pnpm lint` / `pnpm test` | Turbo entry points for cross-workspace builds, linting, and tests. Extend `turbo.json` as needed. |

> Note: The Vite dev server expects generated files in `packages/hn-wasm/pkg`. If you see runtime errors such as “Run `pnpm wasm:build`…”, rebuild the WASM package.

## Current capabilities

- **Web SPA (apps/web)**: Vite + React + TypeScript front-end that loads `@hn/wasm` dynamically, initializes the WASM module once, and renders the top 20 stories with loading/error states.
- **Rust TUI (apps/tui)**: Ratatui split-pane experience. The left pane lists stories; `↑/↓` or `j/k` change selection; the right pane streams comments using background async tasks (via `tokio::spawn` + channel). Comments are sanitized for readability.
- **Shared logic (crates/hn-core)**: A single Rust client powering both targets. Supports story metadata, story text, descendants count, and batched comment fetching with graceful error handling.
- **WASM bridge (crates/hn-wasm + packages/hn-wasm)**: Exposes `fetch_top_posts` along with bindings (`hn_wasm.js`, `.d.ts`). `pnpm dev` keeps these artifacts fresh.
- **Tooling integration**: pnpm workspaces, Turbo pipelines, and Rust workspace are aligned so commands (build, lint, dev) span all packages consistently.

## Workflow tips

1. Run `pnpm dev` and forward port 5174 if you’re on a remote host (Vite listens on `0.0.0.0:5174`).
2. In another terminal, run `pnpm tui` to start the Ratatui client; press `q` or `Esc` to exit.
3. Editing Rust crates triggers the WASM watcher; editing SPA files hot-reloads via Vite.
4. `cargo check -p hn-tui` and `cargo test` remain fast because the WASM build is isolated in the JS workspace.

## Rust workspace overview

- `hn-core`: API client + data types for Hacker News. Auto switches between native/wasm networking, provides batched comment fetching, and reuses serialization for both hosts.
- `hn-wasm`: `cdylib` crate exposing `init_panic_hook` and `fetch_top_posts` via `wasm-bindgen`. Optimized with `wasm-opt -Oz` in release builds.
- `hn-tui`: Binary crate using `tokio`, `crossterm`, and `ratatui` to render the split-pane UI. Uses background tasks with channels to keep the UI responsive.

## Turbo tasks

Turbo coordinates JS + Rust scripts. Current tasks:

- `dev`: persistent task for `apps/web` and `@hn/wasm`.
- `build`, `lint`, `test`: extendable pipelines (currently pass-through). Add `cargo fmt`, `cargo clippy`, etc., as you grow.

## Testing & quality

- `cargo check -p hn-tui` ensures the TUI compiles.
- `pnpm wasm:build`/`pnpm dev` implicitly run `wasm-pack`.
- Pending TODOs cover end-to-end tests, CI, and release automation (see below).

## TODO

- [ ] Implement richer state management in the SPA (pagination, filtering, offline cache).
- [ ] Use `wasmtime` in the TUI to execute the built `hn-wasm` artifact to guarantee the exact same logic as the web bundle.
- [ ] Add CI workflows (GitHub Actions) running `pnpm lint`, `pnpm test`, `cargo fmt --check`, `cargo clippy --all-targets`, `cargo test`, and `pnpm wasm:build`.
- [ ] Layer integration tests that assert the shared Rust logic returns deterministic shapes for fixtures (mock HN API responses).
- [ ] Package release automation (npm + crates.io) once versioning strategy is defined.
- [ ] Document coding standards (formatting, linting, testing) in CONTRIBUTING.md to streamline outside contributions.
