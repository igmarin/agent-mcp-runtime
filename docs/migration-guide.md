# Migration Guide: Skill Ecosystem v6.0

## What Changed

In May 2026, the skill ecosystem was reorganized:
- **10 skills** moved from `rails-agent-skills` to `ruby-core-skills`
- **2 skills** moved from `hanakai-yaku` to `ruby-core-skills`
- **5 new process skills** added to `ruby-core-skills`
- `rails-agent-skills` bumped to v6.0.0
- `hanakai-yaku` bumped to v0.3.0

## Skills That Moved

| Skill | Old Location | New Location |
|-------|-------------|--------------|
| `write-yard-docs` | rails-agent-skills/skills/patterns/ | ruby-core-skills/skills/patterns/ |
| `create-service-object` | rails-agent-skills/skills/patterns/ | ruby-core-skills/skills/patterns/ |
| `implement-calculator-pattern` | rails-agent-skills/skills/patterns/ | ruby-core-skills/skills/patterns/ |
| `integrate-api-client` | rails-agent-skills/skills/api/ | ruby-core-skills/skills/patterns/ |
| `define-domain-language` | rails-agent-skills/skills/ddd/ | ruby-core-skills/skills/ddd/ |
| `review-domain-boundaries` | rails-agent-skills/skills/ddd/ | ruby-core-skills/skills/ddd/ |
| `model-domain` | rails-agent-skills/skills/ddd/ | ruby-core-skills/skills/ddd/ |
| `triage-bug` | rails-agent-skills/skills/testing/ | ruby-core-skills/skills/testing/ |
| `respond-to-review` | rails-agent-skills/skills/code-quality/ | ruby-core-skills/skills/code-quality/ |
| `skill-router` | rails-agent-skills/skills/orchestration/ | ruby-core-skills/skills/orchestration/ |
| `refactor-code` | hanakai-yaku/skills/ | Replaced by `refactor-process` in ruby-core-skills |
| `plan-tests` | hanakai-yaku/skills/ | Replaced by `test-planning-process` in ruby-core-skills |

## New Process Skills (in ruby-core-skills)

| Skill | Purpose |
|-------|---------|
| `tdd-process` | Red-Green-Refactor gates and checkpoints |
| `refactor-process` | Safe refactoring discipline (characterization tests first) |
| `review-process` | Code review severity levels and re-review criteria |
| `security-review-process` | OWASP-based security review checklist |
| `test-planning-process` | Test type decision framework (unit vs integration vs e2e) |

## What You Need To Do

### 1. Install ruby-core-skills

If you use `rails-agent-skills` or `hanakai-yaku`, you now also need `ruby-core-skills`:

```bash
# Via agent-mcp-runtime (automatic — core is always loaded)
agent-mcp-runtime --task "..."

# Via direct install
gh skill install igmarin/ruby-core-skills
```

### 2. Update Local Copies

**Important:** If you have copied skill catalogs into your project (e.g., `.claude/CLAUDE.md`, project-level `AGENTS.md`, or `.cursorrules`), those local copies may reference skills that have moved.

Run this to find stale references:

```bash
grep -rn "skills/ddd/\|skills/patterns/write-yard-docs\|skills/patterns/create-service-object\|skills/api/integrate-api-client\|skills/orchestration/skill-router\|skills/testing/triage-bug\|skills/code-quality/respond-to-review" \
  .claude/ AGENTS.md .cursorrules 2>/dev/null
```

If any results appear, update those references to point to `ruby-core-skills` or remove the path prefix (the runtime resolves by name automatically).

### 3. Deprecation Aliases (Automatic)

Old skill names still work. Both `rails-agent-skills` and `hanakai-yaku` have `deprecated_skills` entries in their `tile.json` that transparently redirect to the new location. You'll see a warning in stderr when a deprecated name is used:

```
⚠ DEPRECATED: 'write-yard-docs' has moved to ruby-core-skills. Use the canonical name.
```

These aliases will be removed in a future major version (v7.0 / v0.4.0).
