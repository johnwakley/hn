use futures::future::join_all;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_BASE_URL: &str = "https://hacker-news.firebaseio.com/v0";
const TOP_STORIES_PATH: &str = "/topstories.json";
const ITEM_PATH: &str = "/item/";

pub type Result<T> = std::result::Result<T, HnError>;

#[derive(Debug, Error)]
pub enum HnError {
    #[error("http request failed: {0}")]
    Http(String),
    #[error("deserialisation failed: {0}")]
    Deserialize(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HackerNewsItem {
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub by: String,
    #[serde(default)]
    pub score: i64,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub time: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct HackerNewsClient {
    base_url: String,
}

impl Default for HackerNewsClient {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }
}

impl HackerNewsClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    pub async fn fetch_top_stories(&self, limit: usize) -> Result<Vec<HackerNewsItem>> {
        let ids = self.fetch_top_story_ids().await?;
        let requested = ids.into_iter().take(limit.max(1)).collect::<Vec<_>>();

        let futures = requested.into_iter().map(|id| self.fetch_item(id));
        let results = join_all(futures).await;

        let mut items = Vec::new();
        for res in results {
            match res {
                Ok(item) => items.push(item),
                Err(err) => tracing::warn!(?err, "skipping item fetch failure"),
            }
        }

        Ok(items)
    }

    async fn fetch_top_story_ids(&self) -> Result<Vec<u64>> {
        let url = format!("{}{}", self.base_url, TOP_STORIES_PATH);
        http_get_json::<Vec<u64>>(&url).await
    }

    async fn fetch_item(&self, id: u64) -> Result<HackerNewsItem> {
        let url = format!(
            "{base}{item_path}{id}.json",
            base = self.base_url,
            item_path = ITEM_PATH,
            id = id
        );
        http_get_json::<HackerNewsItem>(&url).await
    }
}

async fn http_get_json<T>(url: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    #[cfg(target_arch = "wasm32")]
    {
        use gloo_net::http::Request;
        let response = Request::get(url)
            .send()
            .await
            .map_err(|err| HnError::Http(err.to_string()))?;
        response
            .json::<T>()
            .await
            .map_err(|err| HnError::Deserialize(err.to_string()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let response = reqwest::get(url)
            .await
            .map_err(|err| HnError::Http(err.to_string()))?;
        response
            .json::<T>()
            .await
            .map_err(|err| HnError::Deserialize(err.to_string()))
    }
}
