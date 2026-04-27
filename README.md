# sensei docs  先生

Private design documentation, database schema, and project configuration for sensei.

## Structure

| Directory | Contents |
|-----------|----------|
| `database/` | DDL schema — enums, tables, views, procedures, policies, seed data |
| `docs/ideas/` | Feature explorations and proposals (31 numbered ideas) |
| `docs/mockups/` | Interactive JSX prototypes, design summary, data fixtures |
| `docs/journeys/` | User journeys (J1-J9) and system pipelines |
| `docs/design/` | Architecture, schemas, algorithms, ADRs |
| `docs/analysis/` | Gap analysis, skill-command mappings |
| `docs/blueprints/` | Implementation blueprints |
| `.sensei/` | Mindsets, personas, project rules |
| `config/` | Shared config templates |

## Related repos

| Repo | Purpose |
|------|---------|
| [sensei-hq/daemon](https://github.com/sensei-hq/daemon) | Rust daemon, CLI, MCP server + database DDL |
| [sensei-hq/app](https://github.com/sensei-hq/app) | Desktop observatory (Tauri + SvelteKit) |
| [sensei-hq/gateway](https://github.com/sensei-hq/gateway) | LLM routing library |
| [sensei-hq/marketplace](https://github.com/sensei-hq/marketplace) | Skills, commands, agents, templates |
| [sensei-hq/homebrew-tap](https://github.com/sensei-hq/homebrew-tap) | Homebrew formula |
| [sensei-hq/sensei](https://github.com/sensei-hq/sensei) | Marketing website |
| [sensei-hq/corpus](https://github.com/sensei-hq/corpus) | Benchmark corpus |
