//! Binary entry point for the Agent MCP Runtime CLI.

use agent_mcp_runtime::mcp::client::McpClient;
use agent_mcp_runtime::mcp::skill_tools::{
    ListAgentsTool, ListPacksTool, ListSkillsTool, UseAgentTool, UseSkillTool,
};
use agent_mcp_runtime::providers::{LlmProviderFactory, LlmProviderType};
use agent_mcp_runtime::registry::manifest::RegistryManifest;
use agent_mcp_runtime::registry::resolver::RegistryResolver;
use agent_mcp_runtime::registry::source::SkillSourceResolver;
use agent_mcp_runtime::registry::tool::Tool;
use agent_mcp_runtime::registry::PackResolverService;
use agent_mcp_runtime::runner::AgentRunner;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
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

impl Provider {
    const fn to_provider_type(self) -> LlmProviderType {
        match self {
            Self::OpenAI => LlmProviderType::OpenAi,
            Self::Claude => LlmProviderType::Claude,
            Self::Groq => LlmProviderType::Groq,
            Self::Gemini => LlmProviderType::Gemini,
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

    /// Enable verbose debug logging.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Skill pack to load (options: rails, hanami, planning, core).
    /// If omitted, auto-detects from Gemfile.
    #[arg(long, num_args = 1..)]
    pack: Option<Vec<String>>,

    /// Local skill directory to use as highest-priority registry (for development).
    #[arg(long, num_args = 1..)]
    registry: Option<Vec<PathBuf>>,

    /// Path to registry.json manifest. Defaults to bundled registry.
    #[arg(long)]
    registry_manifest: Option<PathBuf>,
}

#[tokio::main]
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

    // Instantiate selected LLM provider using factory service
    let provider = LlmProviderFactory::create(
        args.provider.to_provider_type(),
        &model,
        args.base_url.clone(),
    )?;

    // Determine manifest path
    let manifest_path = args
        .registry_manifest
        .clone()
        .unwrap_or_else(|| PathBuf::from("registry.json"));

    println!(
        "Loading registry manifest from: {}",
        manifest_path.display()
    );
    let manifest_content = std::fs::read_to_string(&manifest_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to read registry manifest at {}: {}",
            manifest_path.display(),
            e
        )
    })?;
    let manifest: RegistryManifest = serde_json::from_str(&manifest_content)?;

    // Query external context providers
    println!("Querying external context providers...");
    let context_registry =
        agent_mcp_runtime::context::ContextProviderRegistry::from_manifest(&manifest);
    let project_context = Arc::new(context_registry.query_all().await?);

    // Git cache source resolver
    let cache_dir = SkillSourceResolver::default_cache_dir()?;
    let source_resolver = SkillSourceResolver::new(cache_dir);

    // Resolve active packs using pack resolver service
    let pack_resolver = PackResolverService::new(&source_resolver);
    let resolver = Arc::new(
        pack_resolver
            .resolve(&manifest, args.pack.as_deref(), args.registry.as_deref())
            .await?,
    );

    // Warn on missing dependencies
    let warnings = resolver.validate_dependencies();
    for warning in &warnings {
        eprintln!("⚠ Dependency warning: {warning}");
    }

    let mut runner = AgentRunner::new(provider, args.max_steps, args.verbose);

    // Register all tools (built-in skills and external MCP client tools)
    register_tools(
        &mut runner,
        Arc::clone(&resolver),
        Arc::clone(&project_context),
        args.mcp_command,
        args.mcp_args,
    )
    .await?;

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

/// Helper function to register all built-in skills and external MCP server tools with the runner.
async fn register_tools(
    runner: &mut AgentRunner,
    resolver: Arc<RegistryResolver>,
    project_context: Arc<agent_mcp_runtime::context::project_context::ProjectContext>,
    mcp_command: Option<String>,
    mcp_args: Option<Vec<String>>,
) -> Result<(), anyhow::Error> {
    // Register MCP skill tools
    runner.register_tool(Box::new(ListSkillsTool {
        resolver: Arc::clone(&resolver),
    }));
    runner.register_tool(Box::new(UseSkillTool {
        resolver: Arc::clone(&resolver),
    }));
    runner.register_tool(Box::new(ListAgentsTool {
        resolver: Arc::clone(&resolver),
    }));
    runner.register_tool(Box::new(UseAgentTool {
        resolver: Arc::clone(&resolver),
    }));
    runner.register_tool(Box::new(ListPacksTool {
        resolver: Arc::clone(&resolver),
    }));
    runner.register_tool(Box::new(
        agent_mcp_runtime::mcp::skill_tools::GetProjectContextTool {
            context: Arc::clone(&project_context),
        },
    ));

    // Spawn MCP Client subprocess if command is given
    if let Some(mcp_cmd) = mcp_command {
        println!("Launching MCP Server subprocess: {mcp_cmd}");

        let mcp_args_ref: Vec<&str> = mcp_args
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

    Ok(())
}
