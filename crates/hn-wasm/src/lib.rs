use hn_core::HackerNewsClient;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub async fn fetch_top_posts(limit: u32) -> Result<JsValue, JsValue> {
    let client = HackerNewsClient::default();
    let posts = client
        .fetch_top_stories(limit as usize)
        .await
        .map_err(|err| JsValue::from_str(&err.to_string()))?;

    serde_wasm_bindgen::to_value(&posts).map_err(|err| JsValue::from_str(&err.to_string()))
}
