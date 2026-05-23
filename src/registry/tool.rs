//! Tool trait definitions and mock implementations for testing.

use async_trait::async_trait;

/// A trait that defines a tool that can be registered and called by the ReAct runner.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the unique name of the tool.
    fn name(&self) -> &str;

    /// Returns a description of the tool (used by the LLM in the system prompt).
    fn description(&self) -> &str;

    /// Calls the tool with the given raw string input, returning the response as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if tool execution fails.
    async fn call(&self, input: &str) -> Result<String, anyhow::Error>;
}

/// A Mock tool implementation for testing purposes.
#[cfg(test)]
pub struct MockTool {
    /// The unique name of the mock tool.
    pub name: String,
    /// The description of the mock tool.
    pub description: String,
    /// The expected response output when this tool is executed.
    pub response: String,
}

#[cfg(test)]
#[async_trait]
impl Tool for MockTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn call(&self, _input: &str) -> Result<String, anyhow::Error> {
        Ok(self.response.clone())
    }
}
