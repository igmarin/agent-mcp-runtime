//! Asynchronous LLM provider implementation for Anthropic's Claude API.

use crate::providers::LlmProvider;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: i64,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Option<Vec<ClaudeContentItem>>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContentItem {
    #[serde(rename = "type")]
    item_type: String,
    text: Option<String>,
}

/// Large Language Model provider implementation using Anthropic's Claude Messages API.
pub struct ClaudeProvider {
    /// Internal reqwest HTTP client for pooled connection reuse.
    client: reqwest::Client,
    /// Anthropic API key.
    api_key: String,
    /// Model name to target (e.g. `claude-3-5-sonnet-20241022`).
    model: String,
    /// Base URL of the API server (useful for mocking).
    base_url: String,
}

impl ClaudeProvider {
    /// Creates a new `ClaudeProvider` with default Anthropic endpoints.
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: "https://api.anthropic.com".to_string(),
        }
    }

    /// Creates a new `ClaudeProvider` with a custom base URL (useful for mocking/testing).
    #[must_use]
    pub fn with_base_url(api_key: String, model: String, base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url,
        }
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn ask_llm(&self, prompt: &str) -> Result<String, anyhow::Error> {
        let url = format!("{}/v1/messages", self.base_url);

        let req_body = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: 4096,
            messages: vec![ClaudeMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&req_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Claude API request failed with status {status}: {err_text}");
        }

        let resp_body: ClaudeResponse = response.json().await?;

        let text = resp_body
            .content
            .and_then(|c| c.into_iter().next())
            .filter(|item| item.item_type == "text")
            .and_then(|item| item.text)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse text content from Claude response"))?;

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_claude_provider_success() -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let mock_server_url = format!("http://{addr}");

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0; 1024];
                let _ = stream.read(&mut buf).await;

                let response_body = r#"{
                    "content": [
                        {
                            "type": "text",
                            "text": "Hello from mock Claude!"
                        }
                    ]
                }"#;
                let http_response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                let _ = stream.write_all(http_response.as_bytes()).await;
            }
        });

        let provider = ClaudeProvider::with_base_url(
            "dummy_key".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            mock_server_url,
        );

        let response = provider.ask_llm("Say hello").await?;
        assert_eq!(response, "Hello from mock Claude!");
        Ok(())
    }
}
