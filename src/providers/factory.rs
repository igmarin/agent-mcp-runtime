//! Service factory for instantiating and configuring LLM providers dynamically.

use crate::providers::{ClaudeProvider, GeminiProvider, GroqProvider, LlmProvider, OpenAiProvider};

/// Supported LLM Provider Types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProviderType {
    /// OpenAI-compatible provider.
    OpenAi,
    /// Anthropic Claude provider.
    Claude,
    /// Groq provider.
    Groq,
    /// Google Gemini provider.
    Gemini,
}

/// A service factory responsible for instantiating and configuring LLM providers.
pub struct LlmProviderFactory;

impl LlmProviderFactory {
    /// Creates a boxed `LlmProvider` based on the specified provider type, target model,
    /// and optional base URL override.
    ///
    /// # Arguments
    ///
    /// * `provider_type` - The target LLM provider type.
    /// * `model` - The name of the target model.
    /// * `base_url` - An optional custom API base URL (e.g. for proxying, `OpenRouter`, or `Ollama`).
    ///
    /// # Returns
    ///
    /// Returns a boxed implementation of [`LlmProvider`] dynamic trait object.
    ///
    /// # Errors
    ///
    /// Returns an error if the required environment variable for the selected provider
    /// is missing or empty.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use agent_mcp_runtime::providers::factory::{LlmProviderFactory, LlmProviderType};
    ///
    /// std::env::set_var("GEMINI_API_KEY", "your-api-key");
    /// let provider = LlmProviderFactory::create(
    ///     LlmProviderType::Gemini,
    ///     "gemini-1.5-flash",
    ///     None,
    /// ).unwrap();
    /// ```
    pub fn create(
        provider_type: LlmProviderType,
        model: &str,
        base_url: Option<String>,
    ) -> Result<Box<dyn LlmProvider + Send + Sync>, anyhow::Error> {
        let model_string = model.to_string();
        match provider_type {
            LlmProviderType::OpenAi => {
                let api_key = get_api_key("OPENAI_API_KEY")?;
                let provider = base_url.map_or_else(
                    || OpenAiProvider::new(api_key.clone(), model_string.clone()),
                    |url| OpenAiProvider::with_base_url(api_key.clone(), model_string.clone(), url),
                );
                Ok(Box::new(provider))
            }
            LlmProviderType::Claude => {
                let api_key = get_api_key("ANTHROPIC_API_KEY")?;
                let provider = base_url.map_or_else(
                    || ClaudeProvider::new(api_key.clone(), model_string.clone()),
                    |url| ClaudeProvider::with_base_url(api_key.clone(), model_string.clone(), url),
                );
                Ok(Box::new(provider))
            }
            LlmProviderType::Groq => {
                let api_key = get_api_key("GROQ_API_KEY")?;
                let provider = base_url.map_or_else(
                    || GroqProvider::new(api_key.clone(), model_string.clone()),
                    |url| GroqProvider::with_base_url(api_key.clone(), model_string.clone(), url),
                );
                Ok(Box::new(provider))
            }
            LlmProviderType::Gemini => {
                let api_key = get_api_key("GEMINI_API_KEY")?;
                let provider = base_url.map_or_else(
                    || GeminiProvider::new(api_key.clone(), model_string.clone()),
                    |url| GeminiProvider::with_base_url(api_key.clone(), model_string.clone(), url),
                );
                Ok(Box::new(provider))
            }
        }
    }
}

// Helper to read and validate the API key environment variable.
fn get_api_key(env_var: &str) -> Result<String, anyhow::Error> {
    let api_key = std::env::var(env_var)
        .map_err(|_| anyhow::anyhow!("{env_var} environment variable is not set"))?
        .trim()
        .to_string();
    if api_key.is_empty() {
        anyhow::bail!("{env_var} environment variable is empty");
    }
    Ok(api_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_missing_env_vars() {
        std::env::remove_var("OPENAI_API_KEY");
        let res = LlmProviderFactory::create(LlmProviderType::OpenAi, "gpt-4o", None);
        assert!(res.is_err());
        assert_eq!(
            res.err().expect("expected an error").to_string(),
            "OPENAI_API_KEY environment variable is not set"
        );

        std::env::set_var("OPENAI_API_KEY", "  ");
        let res = LlmProviderFactory::create(LlmProviderType::OpenAi, "gpt-4o", None);
        assert!(res.is_err());
        assert_eq!(
            res.err().expect("expected an error").to_string(),
            "OPENAI_API_KEY environment variable is empty"
        );
    }

    #[test]
    fn test_factory_success_creation() {
        std::env::set_var("GEMINI_API_KEY", "dummy_key");
        let res = LlmProviderFactory::create(LlmProviderType::Gemini, "gemini-1.5-flash", None);
        assert!(res.is_ok());
    }
}
