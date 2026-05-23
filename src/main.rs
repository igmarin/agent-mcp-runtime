//! Binary entry point for the Agent MCP Runtime CLI.

use agent_mcp_runtime::mcp::client::McpClient;
use agent_mcp_runtime::providers::{
    ClaudeProvider, GeminiProvider, GroqProvider, LlmProvider, OpenAiProvider,
};
use agent_mcp_runtime::registry::tool::Tool;
use agent_mcp_runtime::runner::AgentRunner;
use clap::{Parser, ValueEnum};
use std::sync::Arc;

/// LLM provider to use (options: gemini, openai, claude, groq).
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum Provider {
    #[clap(name = "openai")]
    OpenAI,
    Claude,
    Groq,
    Gemini,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAI => write!(f, "openai"),
            Self::Claude => write!(f, "claude"),
            Self::Groq => write!(f, "groq"),
            Self::Gemini => write!(f, "gemini"),
        }
    }
}

/// CLI Arguments for the `ReAct` MCP Runtime.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The task prompt description for the agent to execute.
    #[arg(short, long)]
    task: String,

    /// Executable command of the Model Context Protocol (MCP) server subprocess to spawn.
    #[arg(short = 'c', long)]
    mcp_command: Option<String>,

    /// Additional arguments to pass to the MCP server subprocess.
    #[arg(short = 'a', long, num_args = 1..)]
    #[allow(clippy::struct_field_names)]
    mcp_args: Option<Vec<String>>,

    /// Limit of `ReAct` reasoning loops.
    #[arg(short = 's', long, default_value_t = 5)]
    max_steps: usize,

    /// LLM provider to use (options: gemini, openai, claude, groq).
    #[arg(short, long, default_value_t = Provider::Gemini)]
    provider: Provider,

    /// Model name to target. Defaults are selected automatically depending on the provider.
    #[arg(short = 'd', long, default_value = "")]
    model: String,

    /// Custom base URL override (useful for connecting OpenRouter/Ollama to the openai provider).
    #[arg(short = 'u', long)]
    base_url: Option<String>,
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let provider_name = args.provider.to_string();
    let model = if args.model.is_empty() {
        match args.provider {
            Provider::OpenAI => "gpt-4o".to_string(),
            Provider::Claude => "claude-3-5-sonnet-20241022".to_string(),
            Provider::Groq => "llama3-8b-8192".to_string(),
            Provider::Gemini => "gemini-1.5-flash".to_string(),
        }
    } else {
        args.model.clone()
    };

    println!("Starting agent session...");
    println!("Task: {}", args.task);
    println!("Provider: {provider_name}");
    println!("Model: {model}");

    // Instantiate selected LLM provider dynamically
    let provider: Box<dyn LlmProvider + Send + Sync> = match args.provider {
        Provider::OpenAI => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable is not set"))?
                .trim()
                .to_string();
            if api_key.is_empty() {
                anyhow::bail!("OPENAI_API_KEY environment variable is empty");
            }
            if let Some(url) = args.base_url {
                Box::new(OpenAiProvider::with_base_url(api_key, model, url))
            } else {
                Box::new(OpenAiProvider::new(api_key, model))
            }
        }
        Provider::Claude => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY environment variable is not set"))?
                .trim()
                .to_string();
            if api_key.is_empty() {
                anyhow::bail!("ANTHROPIC_API_KEY environment variable is empty");
            }
            if let Some(url) = args.base_url {
                Box::new(ClaudeProvider::with_base_url(api_key, model, url))
            } else {
                Box::new(ClaudeProvider::new(api_key, model))
            }
        }
        Provider::Groq => {
            let api_key = std::env::var("GROQ_API_KEY")
                .map_err(|_| anyhow::anyhow!("GROQ_API_KEY environment variable is not set"))?
                .trim()
                .to_string();
            if api_key.is_empty() {
                anyhow::bail!("GROQ_API_KEY environment variable is empty");
            }
            if let Some(url) = args.base_url {
                Box::new(GroqProvider::with_base_url(api_key, model, url))
            } else {
                Box::new(GroqProvider::new(api_key, model))
            }
        }
        Provider::Gemini => {
            let api_key = std::env::var("GEMINI_API_KEY")
                .map_err(|_| anyhow::anyhow!("GEMINI_API_KEY environment variable is not set"))?
                .trim()
                .to_string();
            if api_key.is_empty() {
                anyhow::bail!("GEMINI_API_KEY environment variable is empty");
            }
            if let Some(url) = args.base_url {
                Box::new(GeminiProvider::with_base_url(api_key, model, url))
            } else {
                Box::new(GeminiProvider::new(api_key, model))
            }
        }
    };

    let mut runner = AgentRunner::new(provider, args.max_steps);

    // Spawn MCP Client if command is given
    if let Some(mcp_cmd) = args.mcp_command {
        println!("Launching MCP Server subprocess: {mcp_cmd}");

        let mcp_args_ref: Vec<&str> = args
            .mcp_args
            .as_ref()
            .map_or_else(Vec::new, |v| v.iter().map(AsRef::as_ref).collect());

        let client = Arc::new(McpClient::start(&mcp_cmd, &mcp_args_ref)?);

        println!("Discovering tools from MCP server...");
        let tools = client.get_tools().await?;
        println!("Discovered {} tools.", tools.len());

        for tool in tools {
            println!("  Registered tool: {}", tool.name());
            runner.register_tool(Box::new(tool));
        }
    }

    println!("\nExecuting ReAct Loop...");
    match runner.run(&args.task).await {
        Ok(answer) => {
            println!("\n=================================");
            println!("FINAL ANSWER:");
            println!("{answer}");
            println!("=================================");
        }
        Err(err) => {
            return Err(err);
        }
    }

    Ok(())
}
