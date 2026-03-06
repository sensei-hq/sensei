# sensei

A universal AI skills library and toolchain that helps AI agents work more efficiently on codebases. Scan a repo once, produce structured orientation artifacts, and expose an MCP server so agents load only what they need — fewer tokens, fewer interactions, better results.

## What's here

| Layer | What it does |
|---|---|
| **Skills** | Model-agnostic markdown guidance files installed to `~/.claude/skills` (or equivalent). Teach agents *when* and *how* to use the tools. |
| **MCP server** (`repo-index-server`) | Local server that indexes a repo and serves targeted slices on demand. Offloads deterministic work (generating docs, checking drift, listing exports) so LLMs focus on reasoning. |
| **CLI** (`skills`) | Set up repos, manage developer and company profiles, switch context, install hooks. |

## Core concepts

- **Index once** — `skills index` scans your repo and writes `.llmspec.yaml`, `CLAUDE.md`, `llms.txt`, and `.index/`. Future agents orient in ~500 tokens.
- **Right resolution** — Code is stored at four levels: signature (L0), IO pattern (L1), logic flow (L2), full source (L3). Agents request the minimum needed for the task.
- **Docs stay in sync** — Fingerprinted doc layers (design, code, public) are compared on every commit via a pre-commit hook. Drift is caught before it accumulates.
- **Profiles** — Personal and company profiles carry coding standards, workflow preferences, and skill configuration across all your projects.
- **Benchmarks** — A task corpus measures token usage, interaction counts, and success rate with and without skills, so improvements are evidence-based.

## Repo structure

```
/
├── packages/
│   └── repo-index-server/      MCP server + skills CLI (TypeScript, Bun)
│       ├── src/
│       │   ├── index.ts        MCP server entry
│       │   ├── cli.ts          skills CLI entry
│       │   ├── commands/       CLI command modules
│       │   └── tools/          Shared tool implementations (query, reindex, drift, context…)
│       ├── e2e/                End-to-end tests (*.e2e.ts, Playwright)
│       └── src/**/*.spec.ts    Unit tests (Vitest)
│
├── skills/                     Skill markdown files
│   ├── codebase-indexer/
│   ├── content-compression/
│   ├── agentic-dev-workflow/
│   ├── doc-drift-detector/
│   ├── context-manager/
│   └── benchmark-runner/
│
├── tasks/
│   └── sample.yaml             Benchmark task corpus
│
├── results/                    Benchmark run summaries (*.md committed, *.json gitignored)
│
└── docs/
    ├── features/               What and why — Gherkin scenarios, status tables
    ├── design/                 How — architecture, schemas, algorithms
    └── plans/                  Implementation plans
```

## Getting started

```bash
# Install the CLI globally
bun add -g sensei   # or: npx sensei

# Set up a new repo
cd your-repo
sensei init

# Add to an existing repo without overwriting anything
sensei add

# Check status
sensei status
```

## Development

```bash
# Install dependencies
bun install

# Run unit tests
bun test

# Run e2e tests
bun run test:e2e

# Build
bun run build

# Start MCP server (development)
bun run dev
```

## Docs

- [Features](docs/features/) — what each module does and why
- [Design](docs/design/) — how it works under the hood
- [Implementation plans](docs/plans/) — step-by-step build history
