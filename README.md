# Hacker News Multi-platform Monorepo

This repository hosts a pnpm + Turbo + Cargo monorepo that targets the web and the terminal using one shared Rust implementation that can be compiled to WebAssembly. The main goal is to display the top Hacker News stories inside a Vite React SPA and a Rust TUI built with [ratatui](https://github.com/ratatui/ratatui).

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

### 3. Build the WebAssembly module

```sh
pnpm wasm:build
```

This compiles `crates/hn-wasm` and drops the artifacts into `packages/hn-wasm/pkg`, making them available to the Vite app (and any other JS consumer). Re-run the command whenever the Rust code changes. Turbo can watch this as part of the build pipeline if you wire it into CI.

> Note: The Vite dev server expects the generated files in `packages/hn-wasm/pkg`. If you skip this step the SPA will fail to resolve `@hn/wasm`. When you run `pnpm dev`, Turbo also starts a watcher in `@hn/wasm` so edits to the Rust crates trigger `wasm-pack build --watch` automatically.

### 4. Start the apps

```sh
pnpm dev    # runs the Vite SPA and the WASM watcher (@hn/wasm)
pnpm tui    # launches the Ratatui client via cargo
```

The TUI runs in its own terminal because it needs an interactive TTY; keeping it separate prevents Turbo from killing the SPA dev server when the TUI task exits.

## Rust workspace

- `hn-core` contains the data contracts and fetching/orchestration logic. It automatically switches between `reqwest` (native) and `gloo-net` (wasm) so the same code can target both hosts.
- `hn-wasm` wraps `hn-core` with `wasm-bindgen` exports and handles JSON <-> JS conversion via `serde-wasm-bindgen`.
- `hn-tui` depends directly on `hn-core` today for ergonomics but is structured so you can swap in a `wasmtime` invocation of the compiled `hn-wasm` output if you need to embed the literal WebAssembly module.

## Turbo tasks

Turbo coordinates the dev/build/lint/test pipelines across JS and Rust workspaces. Extend `turbo.json` with additional tasks (e.g., `cargo fmt`, `pnpm wasm:build`, integration tests) as the project grows.

## Next steps

- Implement richer state management in the SPA (pagination, filtering).
- Use `wasmtime` in the TUI to execute the `hn-wasm` artifact directly if you want the binary to literally embed the same `.wasm` module used on the web.
- Add CI workflows (GitHub Actions) that run `pnpm lint`, `pnpm test`, `cargo fmt --check`, `cargo clippy --all-targets`, and `cargo test`.
