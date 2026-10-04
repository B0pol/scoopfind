use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::json;

use crate::models::{AzureSearchResponse, ScoopPackage};

pub const DEFAULT_API_KEY: &str = "DC6D2BBE65FC7313F2C52BBD2B0286ED";
pub const API_ENDPOINT: &str =
    "https://scoopsearch.search.windows.net/indexes/apps/docs/search?api-version=2020-06-30";
pub const SCOOP_BASE_URL: &str = "https://scoop.sh";

#[derive(Clone, Debug)]
pub struct ApiClient {
    client: reqwest::Client,
    pub api_key: String,
    pub key_refreshed: bool,
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiClient {
    pub fn new() -> Self {
        let api_key = Self::load_cached_key().unwrap_or_else(|| DEFAULT_API_KEY.to_string());
        Self {
            client: reqwest::Client::builder()
                .user_agent("scoopfind/0.1.0")
                .build()
                .unwrap_or_default(),
            api_key,
            key_refreshed: false,
        }
    }

    /// Path to store dynamically scraped keys between runs
    fn cache_path() -> PathBuf {
        std::env::temp_dir().join("scoopfind_cached_key.txt")
    }

    fn load_cached_key() -> Option<String> {
        if let Ok(env_key) = std::env::var("SCOOPFIND_API_KEY") {
            let trimmed = env_key.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }

        if let Ok(content) = fs::read_to_string(Self::cache_path()) {
            let trimmed = content.trim();
            if trimmed.len() == 32 {
                return Some(trimmed.to_string());
            }
        }
        None
    }

    fn save_cached_key(key: &str) {
        let _ = fs::write(Self::cache_path(), key);
    }

    /// Executes search against Azure Cognitive Search.
    /// If an authentication error (401 or 403) occurs, it automatically triggers
    /// Milestone 5 dynamic key auto-healing from scoop.sh and retries once.
    pub async fn search(
        &mut self,
        query: &str,
        top: usize,
        skip: usize,
    ) -> Result<Vec<ScoopPackage>> {
        match self.raw_query(query, top, skip).await {
            Ok(packages) => Ok(packages),
            Err(err) => {
                let err_str = format!("{:#}", err);
                if err_str.contains("AUTH_ERROR_401") || err_str.contains("AUTH_ERROR_403") {
                    // Trigger dynamic auto-healing
                    self.refresh_key_from_scoop_sh().await?;
                    self.key_refreshed = true;
                    self.raw_query(query, top, skip).await
                } else {
                    Err(err)
                }
            }
        }
    }

    async fn raw_query(
        &self,
        query: &str,
        top: usize,
        skip: usize,
    ) -> Result<Vec<ScoopPackage>> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "api-key",
            HeaderValue::from_str(&self.api_key)
                .context("Invalid characters in search API key")?,
        );

        let body = json!({
            "search": query.trim(),
            "searchMode": "all",
            "count": true,
            "top": top,
            "skip": skip,
            "select": "Id,Name,Description,Notes,Homepage,License,Version,Metadata/Repository,Metadata/OfficialRepository,Metadata/RepositoryStars,Metadata/FilePath,Metadata/Committed,Metadata/Sha"
        });

        let resp = self
            .client
            .post(API_ENDPOINT)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .context("Failed to send HTTP request to Scoop Search API")?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            bail!("AUTH_ERROR_401: Unauthorized API key");
        }
        if status == reqwest::StatusCode::FORBIDDEN {
            bail!("AUTH_ERROR_403: Forbidden API key");
        }
        if !status.is_success() {
            let error_text = resp.text().await.unwrap_or_default();
            bail!("API returned HTTP status {}: {}", status, error_text);
        }

        let parsed: AzureSearchResponse = resp
            .json()
            .await
            .context("Failed to parse JSON response from Azure Search")?;

        Ok(parsed.value)
    }

    /// Milestone 5: Dynamically scrapes the active Azure Search key from scoop.sh frontend bundle
    pub async fn refresh_key_from_scoop_sh(&mut self) -> Result<String> {
        // 1. Fetch scoop.sh index HTML
        let html = self
            .client
            .get(SCOOP_BASE_URL)
            .send()
            .await
            .context("Failed to connect to scoop.sh to refresh API key")?
            .text()
            .await
            .context("Failed to read HTML body from scoop.sh")?;

        // 2. Locate the main javascript bundle (e.g. /assets/index-CPTR681n.js)
        let script_regex =
            Regex::new(r#"src="(/assets/index-[^"]+\.js)""#).context("Invalid script regex")?;
        let captures = script_regex.captures(&html).context(
            "Could not locate JavaScript bundle script tag on scoop.sh website",
        )?;
        let script_path = captures
            .get(1)
            .context("Missing regex capture group for asset path")?
            .as_str();

        let script_url = format!("{}{}", SCOOP_BASE_URL, script_path);

        // 3. Fetch the javascript bundle
        let js = self
            .client
            .get(&script_url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch JavaScript bundle: {}", script_url))?
            .text()
            .await
            .context("Failed to read script contents")?;

        // 4. Extract VITE_APP_AZURESEARCH_KEY
        let key_regex = Regex::new(r#"VITE_APP_AZURESEARCH_KEY:\s*[`'"]([A-Fa-f0-9]{32})[`'"]"#)
            .context("Invalid key regex")?;
        let key_captures = key_regex.captures(&js).context(
            "Could not extract VITE_APP_AZURESEARCH_KEY from scoop.sh bundle",
        )?;
        let new_key = key_captures
            .get(1)
            .context("Missing key regex capture")?
            .as_str()
            .to_string();

        self.api_key = new_key.clone();
        Self::save_cached_key(&new_key);
        Ok(new_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_live_search_query() {
        let mut client = ApiClient::new();
        let results = client.search("ripgrep", 3, 0).await;
        assert!(results.is_ok(), "Expected search query to succeed: {:?}", results.err());
        let packages = results.unwrap();
        assert!(!packages.is_empty(), "Expected at least 1 package result for 'ripgrep'");
        assert_eq!(packages[0].name.to_lowercase(), "ripgrep");
    }

    #[tokio::test]
    async fn test_extract_dynamic_key() {
        let mut client = ApiClient::new();
        let key_result = client.refresh_key_from_scoop_sh().await;
        assert!(key_result.is_ok(), "Expected dynamic key extraction to succeed");
        let key = key_result.unwrap();
        assert_eq!(key.len(), 32, "Key should be a 32-character hexadecimal string");
    }

    #[tokio::test]
    async fn test_auto_heal_with_bad_key() {
        let mut client = ApiClient::new();
        // Force an invalid key to trigger Milestone 5 auto-healing
        client.api_key = "00000000000000000000000000000000".to_string();
        let results = client.search("curl", 2, 0).await;
        assert!(results.is_ok(), "Expected auto-healing to refresh key and succeed");
        assert!(client.key_refreshed, "Expected key_refreshed flag to be true");
    }
}
