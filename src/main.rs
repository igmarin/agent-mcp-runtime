//! Binary entry point for the Agent MCP Runtime CLI.

use agent_mcp_runtime::mcp::client::McpClient;
use agent_mcp_runtime::providers::{
    ClaudeProvider, GeminiProvider, GroqProvider, LlmProvider, OpenAiProvider,
};
use agent_mcp_runtime::registry::tool::Tool;
use agent_mcp_runtime::runner::AgentRunner;
use clap::Parser;
use std::sync::Arc;

/// CLI Arguments for the ReAct MCP Runtime.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The task prompt description for the agent to execute.
    #[arg(short, long)]
    task: String,

    /// Executable command of the Model Context Protocol (MCP) server subprocess to spawn.
    #[arg(short, long)]
    mcp_command: Option<String>,

    /// Additional arguments to pass to the MCP server subprocess.
    #[arg(short, long, num_args = 1..)]
    mcp_args: Option<Vec<String>>,

    /// Limit of ReAct reasoning loops.
    #[arg(short, long, default_value_t = 5)]
    max_steps: usize,

    /// LLM provider to use (options: gemini, openai, claude, groq).
    #[arg(short, long, default_value = "gemini")]
    provider: String,

    /// Model name to target. Defaults are selected automatically depending on the provider.
    #[arg(short = 'd', long, default_value = "")]
    model: String,

    /// Custom base URL override (useful for connecting OpenRouter/Ollama to the openai provider).
    #[arg(short = 'u', long)]
    base_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let provider_name = args.provider.to_lowercase();
    let model = if args.model.is_empty() {
        match provider_name.as_str() {
            "openai" => "gpt-4o".to_string(),
            "claude" => "claude-3-5-sonnet-20241022".to_string(),
            "groq" => "llama3-8b-8192".to_string(),
            _ => "gemini-1.5-flash".to_string(),
        }
    } else {
        args.model.clone()
    };

    println!("Starting agent session...");
    println!("Task: {}", args.task);
    println!("Provider: {}", provider_name);
    println!("Model: {}", model);

    // Instantiate selected LLM provider dynamically
    let provider: Box<dyn LlmProvider + Send + Sync> = match provider_name.as_str() {
        "openai" => {
            let api_key = match std::env::var("OPENAI_API_KEY") {
                Ok(key) if !key.trim().is_empty() => key,
                _ => {
                    eprintln!("Error: OPENAI_API_KEY environment variable is not set or empty.");
                    std::process::exit(1);
                }
            };
            if let Some(url) = args.base_url {
                Box::new(OpenAiProvider::with_base_url(api_key, model, url))
            } else {
                Box::new(OpenAiProvider::new(api_key, model))
            }
        }
        "claude" => {
            let api_key = match std::env::var("ANTHROPIC_API_KEY") {
                Ok(key) if !key.trim().is_empty() => key,
                _ => {
                    eprintln!("Error: ANTHROPIC_API_KEY environment variable is not set or empty.");
                    std::process::exit(1);
                }
            };
            if let Some(url) = args.base_url {
                Box::new(ClaudeProvider::with_base_url(api_key, model, url))
            } else {
                Box::new(ClaudeProvider::new(api_key, model))
            }
        }
        "groq" => {
            let api_key = match std::env::var("GROQ_API_KEY") {
                Ok(key) if !key.trim().is_empty() => key,
                _ => {
                    eprintln!("Error: GROQ_API_KEY environment variable is not set or empty.");
                    std::process::exit(1);
                }
            };
            if let Some(url) = args.base_url {
                Box::new(GroqProvider::with_base_url(api_key, model, url))
            } else {
                Box::new(GroqProvider::new(api_key, model))
            }
        }
        _ => {
            let api_key = match std::env::var("GEMINI_API_KEY") {
                Ok(key) if !key.trim().is_empty() => key,
                _ => {
                    eprintln!("Error: GEMINI_API_KEY environment variable is not set or empty.");
                    std::process::exit(1);
                }
            };
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
        println!("Launching MCP Server subprocess: {}", mcp_cmd);

        let mcp_args_ref: Vec<&str> = match &args.mcp_args {
            Some(v) => v.iter().map(AsRef::as_ref).collect(),
            None => Vec::new(),
        };

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
            println!("{}", answer);
            println!("=================================");
        }
        Err(err) => {
            eprintln!("\nExecution failed with error: {}", err);
            std::process::exit(1);
        }
    }

    Ok(())
}
