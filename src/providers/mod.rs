//! Providers module defining the LLM provider trait and test mocks.

use async_trait::async_trait;

pub mod gemini;
pub use gemini::GeminiProvider;

/// Trait representing a generic Large Language Model provider.
#[async_trait]
pub trait LlmProvider {
    /// Asks the LLM a question and returns its text response.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or response deserialization fails.
    async fn ask_llm(&self, prompt: &str) -> Result<String, anyhow::Error>;
}

/// A Mock LLM provider for testing purposes.
#[cfg(test)]
pub struct MockLlmProvider {
    /// The response that the mock provider is expected to return.
    pub expected_response: String,
}

#[cfg(test)]
#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn ask_llm(&self, _prompt: &str) -> Result<String, anyhow::Error> {
        Ok(self.expected_response.clone())
    }
}
