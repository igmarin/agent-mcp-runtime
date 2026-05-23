//! Asynchronous LLM provider implementation for Google's Gemini API.

use crate::providers::LlmProvider;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContentResponse>,
}

#[derive(Debug, Deserialize)]
struct GeminiContentResponse {
    parts: Option<Vec<GeminiPartResponse>>,
}

#[derive(Debug, Deserialize)]
struct GeminiPartResponse {
    text: Option<String>,
}

/// Large Language Model provider implementation using Google's Gemini API.
pub struct GeminiProvider {
    /// Internal reqwest HTTP client for pooled connection reuse.
    client: reqwest::Client,
    /// Google Gemini API key.
    api_key: String,
    /// Model name to target (e.g. `gemini-1.5-flash`).
    model: String,
    /// Base URL of the API server (useful for mocking).
    base_url: String,
}

impl GeminiProvider {
    /// Creates a new `GeminiProvider` instance with default Google endpoints.
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: "https://generativelanguage.googleapis.com".to_string(),
        }
    }

    /// Creates a new `GeminiProvider` with a custom base URL (useful for mocking/testing).
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
impl LlmProvider for GeminiProvider {
    async fn ask_llm(&self, prompt: &str) -> Result<String, anyhow::Error> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url, self.model, self.api_key
        );

        let req_body = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart {
                    text: prompt.to_string(),
                }],
            }],
        };

        let response = self.client.post(&url).json(&req_body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = match response.text().await {
                Ok(t) => t,
                Err(_) => String::new(),
            };
            anyhow::bail!("Gemini API request failed with status {status}: {err_text}");
        }

        let resp_body: GeminiResponse = response.json().await?;

        let text = resp_body
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|cand| cand.content)
            .and_then(|content| content.parts)
            .and_then(|parts| parts.into_iter().next())
            .and_then(|part| part.text)
            .ok_or_else(|| {
                anyhow::anyhow!("Failed to parse text from Gemini response candidates")
            })?;

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_gemini_provider_success() -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let mock_server_url = format!("http://{addr}");

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0; 1024];
                let _ = stream.read(&mut buf).await;

                // Return a mock HTTP 200 response with valid Gemini JSON content
                let response_body = r#"{
                    "candidates": [
                        {
                            "content": {
                                "parts": [
                                    {
                                        "text": "Hello from mock Gemini!"
                                    }
                                ]
                            }
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

        let provider = GeminiProvider::with_base_url(
            "dummy_key".to_string(),
            "gemini-1.5-flash".to_string(),
            mock_server_url,
        );

        let response = provider.ask_llm("Say hello").await?;
        assert_eq!(response, "Hello from mock Gemini!");
        Ok(())
    }
}
