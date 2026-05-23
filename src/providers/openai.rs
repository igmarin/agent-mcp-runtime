//! Asynchronous LLM provider implementation for OpenAI-compatible APIs.

use crate::providers::LlmProvider;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Option<Vec<OpenAiChoice>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiMessageResponse>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessageResponse {
    content: Option<String>,
}

/// Large Language Model provider implementation using `OpenAI`'s API (or compatible APIs like `OpenRouter` or Ollama).
pub struct OpenAiProvider {
    /// Internal reqwest HTTP client for pooled connection reuse.
    client: reqwest::Client,
    /// API authentication key.
    api_key: String,
    /// Model name to target.
    model: String,
    /// Base URL of the API server (useful for OpenRouter/Ollama/mocking).
    base_url: String,
}

impl OpenAiProvider {
    /// Creates a new `OpenAiProvider` with default `OpenAI` endpoints.
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
            base_url: "https://api.openai.com".to_string(),
        }
    }

    /// Creates a new `OpenAiProvider` with a custom base URL (useful for `OpenRouter`, Ollama, or mocking).
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
impl LlmProvider for OpenAiProvider {
    async fn ask_llm(&self, prompt: &str) -> Result<String, anyhow::Error> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let req_body = OpenAiRequest {
            model: self.model.clone(),
            messages: vec![OpenAiMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let err_text = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenAI API request failed with status {status}: {err_text}");
        }

        let resp_body: OpenAiResponse = response.json().await?;

        let text = resp_body
            .choices
            .and_then(|c| c.into_iter().next())
            .and_then(|choice| choice.message)
            .and_then(|msg| msg.content)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse text from OpenAI response choices"))?;

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_openai_provider_success() -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let mock_server_url = format!("http://{addr}");

        tokio::spawn(async move {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buf = [0; 1024];
                let _ = stream.read(&mut buf).await;

                let response_body = r#"{
                    "choices": [
                        {
                            "message": {
                                "role": "assistant",
                                "content": "Hello from mock OpenAI!"
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

        let provider = OpenAiProvider::with_base_url(
            "dummy_key".to_string(),
            "gpt-4o".to_string(),
            mock_server_url,
        );

        let response = provider.ask_llm("Say hello").await?;
        assert_eq!(response, "Hello from mock OpenAI!");
        Ok(())
    }
}
