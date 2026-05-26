# Getting Started

This guide walks you through building the codebase, running checks, and writing custom tools or providers for the `agent-mcp-runtime`.

## Building the Codebase

Compile the library target:
```bash
cargo build
```

## Running Guardrail Checks

To satisfy the repository health checks (which run in the GitHub Actions CI pipeline):

1. **Format Check**:
   ```bash
   cargo fmt --check
   ```
   To automatically format the code, run:
   ```bash
   cargo fmt
   ```

2. **Clippy Static Analysis**:
   ```bash
   cargo clippy --all-targets -- -D warnings
   ```
   All warnings must be resolved to compile successfully.

3. **Run Unit and Integration Tests**:
   ```bash
   cargo test --all-targets
   ```

---

## Writing a Custom LLM Provider

To integrate a new LLM provider (such as Gemini, Claude, or OpenRouter), implement the asynchronous `LlmProvider` trait:

```rust
use async_trait::async_trait;
use agent_mcp_runtime::providers::LlmProvider;

pub struct GeminiProvider {
    api_key: String,
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn ask_llm(&self, prompt: &str) -> Result<String, anyhow::Error> {
        // Implement HTTP request using reqwest/ureq to Gemini API
        todo!("Make HTTP POST request to Gemini API and extract response text")
    }
}
```

---

## Writing a Custom Tool

To create a new tool that the agent can execute, implement the async `Tool` trait:

```rust
use async_trait::async_trait;
use agent_mcp_runtime::registry::tool::Tool;

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write text content to a local file. Input format must be a JSON containing: path, content."
    }

    async fn call(&self, input: &str) -> Result<String, anyhow::Error> {
        // Implement logic (e.g. parse input as JSON, write to file)
        Ok("File written successfully".to_string())
    }
}
```

---

## Running the Agent Runner

Assemble your LLM provider and tools inside an `AgentRunner`:

```rust
use agent_mcp_runtime::runner::AgentRunner;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // 1. Initialize provider
    // 1. Initialize provider using environment variables to keep keys secure
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| anyhow::anyhow!("GEMINI_API_KEY environment variable is not set"))?;
    let provider = GeminiProvider::new(api_key, "gemini-1.5-flash".to_string());
    
    // 2. Initialize runner with a maximum execution limit of 5 steps
    let mut runner = AgentRunner::new(Box::new(provider), 5, false);
    
    // 3. Register tools
    runner.register_tool(Box::new(FileWriteTool));
    
    // 4. Run task
    let result = runner.run("Write a file at hello.txt containing 'Hello World'").await?;
    println!("Final Answer: {}", result);
    
    Ok(())
}
```
