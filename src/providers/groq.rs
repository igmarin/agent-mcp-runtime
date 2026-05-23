//! Asynchronous LLM provider implementation for Groq's API.

use crate::providers::LlmProvider;
use crate::providers::OpenAiProvider;
use async_trait::async_trait;

/// Large Language Model provider implementation using Groq's API.
pub struct GroqProvider {
    /// Internal OpenAI provider reused for request serialization and communication.
    inner: OpenAiProvider,
}

impl GroqProvider {
    /// Creates a new `GroqProvider` instance pointing to Groq's official base URL.
    #[must_use]
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            inner: OpenAiProvider::with_base_url(
                api_key,
                model,
                "https://api.groq.com/openai".to_string(),
            ),
        }
    }

    /// Creates a new `GroqProvider` with a custom base URL (useful for mocking/testing).
    #[must_use]
    pub fn with_base_url(api_key: String, model: String, base_url: String) -> Self {
        Self {
            inner: OpenAiProvider::with_base_url(api_key, model, base_url),
        }
    }
}

#[async_trait]
impl LlmProvider for GroqProvider {
    async fn ask_llm(&self, prompt: &str) -> Result<String, anyhow::Error> {
        self.inner.ask_llm(prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_groq_provider_success() -> Result<(), anyhow::Error> {
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
                                "content": "Hello from mock Groq!"
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

        let provider = GroqProvider::with_base_url(
            "dummy_key".to_string(),
            "llama3-8b-8192".to_string(),
            mock_server_url,
        );

        let response = provider.ask_llm("Say hello").await?;
        assert_eq!(response, "Hello from mock Groq!");
        Ok(())
    }
}
