export type HackerNewsItem = {
  id: number;
  title: string;
  by: string;
  score: number;
  url?: string | null;
  time?: number | null;
};

type WasmBindings = typeof import('@hn/wasm');

let wasmModule: Promise<WasmBindings> | null = null;

async function loadWasmModule(): Promise<WasmBindings> {
  if (!wasmModule) {
    wasmModule = import('@hn/wasm').then(async (module) => {
      const init = module.default as (input?: RequestInfo | URL | Response | BufferSource | WebAssembly.Module) => Promise<unknown>;
      await init();
      module.init_panic_hook?.();
      return module;
    });
  }

  return wasmModule;
}

export async function fetchTopPosts(limit = 20): Promise<HackerNewsItem[]> {
  const module = await loadWasmModule();
  const result = await module.fetch_top_posts(limit);
  return result as HackerNewsItem[];
}
