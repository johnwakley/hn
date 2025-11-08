# @hn/wasm

Build the WebAssembly bindings that wrap the shared Rust logic.

```sh
pnpm wasm:build
```

The compiled artifacts land in `packages/hn-wasm/pkg` and are consumed by the Vite SPA and other JavaScript callers.
