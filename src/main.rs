//! Binary entry point for the Agent MCP Runtime CLI.

use agent_mcp_runtime::mcp::client::McpClient;
use agent_mcp_runtime::mcp::skill_tools::{
    ListAgentsTool, ListPacksTool, ListSkillsTool, UseAgentTool, UseSkillTool,
};
use agent_mcp_runtime::providers::{
    ClaudeProvider, GeminiProvider, GroqProvider, LlmProvider, OpenAiProvider,
};
use agent_mcp_runtime::registry::detector::{DetectedFramework, PackDetector};
use agent_mcp_runtime::registry::manifest::RegistryManifest;
use agent_mcp_runtime::registry::resolver::{LoadedPack, RegistryResolver};
use agent_mcp_runtime::registry::source::SkillSourceResolver;
use agent_mcp_runtime::registry::tile::TileManifest;
use agent_mcp_runtime::registry::tool::Tool;
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
    let context_registry = agent_mcp_runtime::context::ContextProviderRegistry::from_manifest(&manifest);
    let project_context = Arc::new(context_registry.query_all().await);

    // Resolve active packs
    let mut active_pack_names = std::collections::BTreeSet::new();
    for (name, pack_def) in &manifest.packs {
        if pack_def.always_loaded.unwrap_or(false) {
            active_pack_names.insert(name.clone());
        }
    }

    if let Some(ref explicit) = args.pack {
        for p in explicit {
            active_pack_names.insert(p.clone());
        }
    } else {
        let detected = PackDetector::detect();
        if detected.is_empty() {
            println!(
                "No framework detected in Gemfile. Loading default stack: {:?}",
                manifest.default_stack
            );
            for p in &manifest.default_stack {
                active_pack_names.insert(p.clone());
            }
        } else {
            println!("Auto-detected frameworks: {detected:?}");
            for framework in detected {
                match framework {
                    DetectedFramework::Rails => {
                        active_pack_names.insert("rails".to_string());
                    }
                    DetectedFramework::Hanami => {
                        active_pack_names.insert("hanami".to_string());
                    }
                }
            }
        }
    }

    // Git cache source resolver
    let cache_dir = SkillSourceResolver::default_cache_dir()?;
    let source_resolver = SkillSourceResolver::new(cache_dir);

    let mut loaded_packs = Vec::new();
    for name in active_pack_names {
        let pack_def = manifest
            .packs
            .get(&name)
            .ok_or_else(|| anyhow::anyhow!("Pack '{name}' not defined in registry manifest"))?;

        println!(
            "Resolving pack '{name}' from source '{}'...",
            pack_def.source
        );
        let base_path = source_resolver.resolve(&pack_def.source).await?;
        let tile_path = base_path.join(&pack_def.tile);
        let tile_content = std::fs::read_to_string(&tile_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read tile manifest for pack '{name}' at {}: {e}",
                tile_path.display()
            )
        })?;
        let tile: TileManifest = serde_json::from_str(&tile_content)?;

        let priority = match name.as_str() {
            "rails" | "hanami" => 10,
            "core" => 20,
            _ => 30,
        };

        loaded_packs.push(LoadedPack {
            name,
            tile,
            base_path,
            priority,
        });
    }

    // Load local registries
    if let Some(ref local_paths) = args.registry {
        for (i, path) in local_paths.iter().enumerate() {
            let tile_path = path.join("tile.json");
            println!("Loading local registry from: {}", tile_path.display());
            let tile_content = std::fs::read_to_string(&tile_path).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read local registry tile manifest at {}: {e}",
                    tile_path.display()
                )
            })?;
            let tile: TileManifest = serde_json::from_str(&tile_content)?;

            loaded_packs.push(LoadedPack {
                name: format!("local_{i}"),
                tile,
                base_path: path.clone(),
                priority: 0, // Highest priority
            });
        }
    }

    let resolver = Arc::new(RegistryResolver::new(loaded_packs));

    // Warn on missing dependencies
    let warnings = resolver.validate_dependencies();
    for warning in &warnings {
        eprintln!("⚠ Dependency warning: {warning}");
    }

    let mut runner = AgentRunner::new(provider, args.max_steps, args.verbose);

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
    runner.register_tool(Box::new(agent_mcp_runtime::mcp::skill_tools::GetProjectContextTool {
        context: Arc::clone(&project_context),
    }));

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
