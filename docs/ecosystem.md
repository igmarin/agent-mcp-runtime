# AI Skill Ecosystem — Overview

## Architecture

```mermaid
graph TD
User / AI Agent
      │
      ▼
agent-mcp-runtime (Rust CLI)
  ├── Registry Resolver (pack priority, deprecation aliases)
  ├── ReAct Runner (thought → action → observation loop)
  └── MCP Server (list_skills, use_skill, list_agents, use_agent, list_packs)
      │
      ▼  reads tile.json from each pack
  ┌───────────┬───────────────────┬──────────────────┬─────────────────────────┐
  │ core      │ rails             │ hanami           │ planning                │
  │ (always)  │ (auto-detect)     │ (auto-detect)    │ (default stack)         │
  └───────────┴───────────────────┴──────────────────┴─────────────────────────┘
  ruby-core    rails-agent         hanakai-yaku       agnostic-planning
  -skills      -skills                                -skills
```

## Repos

### [ruby-core-skills](https://github.com/igmarin/ruby-core-skills)

- **Role:** Shared Ruby skills + process discipline (TDD, refactoring, review, security review, test planning)
- **Skills:** 15 (10 extracted atomic + 5 new process)
- **Agents:** 0 — this is a library, not an orchestration layer
- **URL:** https://github.com/igmarin/ruby-core-skills

### [rails-agent-skills](https://github.com/igmarin/rails-agent-skills)

- **Role:** Rails-specific development skills and agent workflows
- **Skills:** 28 local + 15 from core = 43 available
- **Agents:** 9 (tdd, review, setup, quality, engine, bug-fix, graphql, migration, background-job)
- **Depends on:** ruby-core-skills
- **URL:** https://github.com/igmarin/rails-agent-skills

### [hanakai-yaku](https://github.com/igmarin/hanakai-yaku)

- **Role:** Hanami 2.x / dry-rb / ROM development skills and agent workflows
- **Skills:** 35 local + 15 from core = 50 available
- **Agents:** 10
- **Depends on:** ruby-core-skills
- **URL:** https://github.com/igmarin/hanakai-yaku

### [agnostic-planning-skills](https://github.com/igmarin/agnostic-planning-skills)

- **Role:** Language-agnostic project management and planning
- **Skills:** 10
- **Agents:** 4 (delivery-lead, product-owner, project-manager, tech-lead)
- **URL:** https://github.com/igmarin/agnostic-planning-skills

### [agent-mcp-runtime](https://github.com/igmarin/agent-mcp-runtime)

- **Role:** Rust CLI that composes skills from multiple packs via MCP
- **Auto-detection:** Reads Gemfile to detect Rails/Hanami, loads matching packs
- **CLI flags:** `--pack`, `--registry`, `--registry-manifest`
- **URL:** https://github.com/igmarin/agent-mcp-runtime

### [ruby-skill-bench](https://github.com/igmarin/ruby-skill-bench)

- **Role:** Benchmark/eval engine measuring "ROI of Context" for skills
- **Multi-repo support:** `--skill` flag for cross-repo skill paths
- **URL:** https://github.com/igmarin/ruby-skill-bench

## How It Works

1. User runs: `agent-mcp-runtime --task "Add full_name to User model"`
2. Runtime reads `Gemfile`, detects Rails → loads packs: `core` + `rails`
3. Registry resolver merges skills from both packs (Rails priority > core)
4. LLM uses `list_skills` → `use_skill("write-tests")` → follows instructions
5. Deprecated skill names (e.g., `write-yard-docs` in rails tile.json) redirect transparently

## Pack Resolution Priority

1. `--registry ./local-path` (priority 0 — highest, for development)
2. Framework packs: `rails` or `hanami` (priority 10)
3. `core` (priority 20, always loaded)
4. Other packs: `planning` (priority 30)

Framework skills override core skills of the same name. Local registry overrides everything.

## rails-ai-bridge (Future Integration)

`rails-ai-bridge` is an independent project for Rails introspection. Future integration: `agent-mcp-runtime` connects to its MCP server as a context provider. The `load-context` skill delegates to it when available. No code dependencies today — just documented as an optional accelerator.
