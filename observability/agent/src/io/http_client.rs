//! http_client.rs — Minimal HTTP client wrapper for Observability-Agent
//!
//! The agent does not need a full Rhelma HTTP stack.
//! This wrapper ensures:
//!   - Proper error conversion to AgentError
//!   - Lightweight async usage
//!   - JSON GET/POST helpers

use reqwest::{Client, Response};
use rhelma_http_observability::reqwest::ReqwestRequestBuilderExt;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::AgentError;

/// Minimal HTTP client wrapper for Observability-Agent
#[derive(Clone)]
pub struct HttpClient {
    /// Internal reqwest client
    client: Client,
}

impl HttpClient {
    /// Build a new HTTP client with safe defaults.
    ///
    /// # Returns
    /// A new HTTP client instance
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("rhelma-observability-agent/1.0")
                .build()
                .expect("reqwest client must build"),
        }
    }

    /// Perform a GET request and parse JSON body.
    ///
    /// # Arguments
    /// * `url` - URL to GET
    ///
    /// # Returns
    /// `Result<T, AgentError>` - Parsed JSON response or error
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T, AgentError> {
        let res = self
            .client
            .get(url)
            .with_rhelma_observability()
            .send()
            .await
            .map_err(|e| AgentError::internal(format!("http GET error: {e}")))?;

        Self::parse_json(res).await
    }

    /// Perform a POST request with JSON body and parse JSON response.
    ///
    /// # Arguments
    /// * `url` - URL to POST to
    /// * `body` - JSON body to send
    ///
    /// # Returns
    /// `Result<T, AgentError>` - Parsed JSON response or error
    pub async fn post_json<B, T>(&self, url: &str, body: &B) -> Result<T, AgentError>
    where
        B: Serialize,
        T: DeserializeOwned,
    {
        let res = self
            .client
            .post(url)
            .json(body)
            .with_rhelma_observability()
            .send()
            .await
            .map_err(|e| AgentError::internal(format!("http POST error: {e}")))?;

        Self::parse_json(res).await
    }

    /// Parses JSON response from HTTP response
    ///
    /// # Arguments
    /// * `res` - HTTP response
    ///
    /// # Returns
    /// `Result<T, AgentError>` - Parsed JSON or error
    async fn parse_json<T: DeserializeOwned>(res: Response) -> Result<T, AgentError> {
        let status = res.status();

        if !status.is_success() {
            return Err(AgentError::invalid(format!(
                "upstream responded with HTTP {status}"
            )));
        }

        res.json::<T>()
            .await
            .map_err(|e| AgentError::internal(format!("failed to decode json: {e}")))
    }
}
