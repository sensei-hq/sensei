# Package Adapters — Design

## Problem

The current index (`symbol-map.json`) is a flat file → symbols map. It has no concept of:

- Package boundaries and their purpose
- Which files belong to which package
- How design/feature docs relate to implementation
- Tech stack and tooling per package (critical for monorepos)

This makes it hard to answer questions like "what does `packages/sensei` do?", "what drives the auth implementation?", or "has this design doc drifted from the code it describes?"

---

## Approach: Glob-based adapter discovery

Instead of walking a fixed folder structure, each adapter declares the file names it understands. The scanner runs all adapters' glob patterns across the repo (excluding `.gitignore`d paths and `.sensei/`), groups matches by directory, and runs the matching adapter for each group.

```
repo root
  ↓
glob("**/package.json", "**/README.md", "**/go.mod", "**/Cargo.toml", ...)
  ↓
group results by directory
  ↓
for each directory: run matching adapter(s)
  ↓
emit PackageInfo[] → folder-map.json
```

This is intentionally not tied to a specific folder depth or convention (monorepo, flat, nested — all work the same way).

---

## Adapter interface

```typescript
interface PackageAdapter {
  name: string;                    // "npm", "python", "go", "rust"
  globs: string[];                 // file names this adapter recognises
  detect(files: string[]): boolean; // true if enough signals are present
  extract(dir: string, files: string[], repoPath: string): Promise<PackageInfo>;
}

interface PackageInfo {
  path: string;                    // dir relative to repoPath
  adapter: string;                 // which adapter ran
  name: string;
  description?: string;
  version?: string;
  role?: PackageRole;              // inferred (see below)
  entryPoints: string[];           // main/exports/bin
  stack: string[];                 // inferred from deps
  scripts: Record<string, string>;
  dependencies: Record<string, string>;
  devDependencies: Record<string, string>;
  readme?: ReadmeSummary;
  links: DocLink[];                // all links found in README + other docs
}

type PackageRole = "cli" | "lib" | "app" | "service" | "ui" | "config" | "docs" | "unknown";

interface ReadmeSummary {
  title: string;
  description: string;             // first substantive paragraph
  sections: string[];              // H2/H3 headings
  links: DocLink[];
}

interface DocLink {
  from: string;                    // file containing the link
  to: string;                      // resolved relative path or URL
  text: string;                    // link text
  kind: "doc" | "code" | "external";
}
```

---

## README.md extraction

README files carry structure that is often richer than `package.json` descriptions. We extract:

### What to parse

| Element | What it tells us |
|---------|-----------------|
| H1 | Package/project name (may differ from package.json) |
| First paragraph after H1 | Best human-written description |
| H2/H3 headings | Sections: API, Usage, Configuration, Architecture... |
| `[text](./path)` links | Relationships to other docs or source files |
| Code blocks with shell commands | Usage examples, CLI patterns |
| Bullet lists under "Features" / "Commands" | Feature → implementation hints |

### Link classification

```
[text](./docs/auth.md)        → kind: "doc",  to: "docs/auth.md"
[text](./src/commands/init.ts)→ kind: "code", to: "src/commands/init.ts"
[text](https://example.com)   → kind: "external", skipped for graph
[text](#section)              → kind: "anchor", skipped
```

Local links (doc and code) are normalised to repo-relative paths and recorded in the link graph.

### Why this matters for drift

If a README links to `src/commands/init.ts` and that file changes, the README is a candidate for drift — it might describe behaviour that no longer matches. This extends the traceability matrix beyond explicit `@covers` tags to any doc that *references* a file.

---

## JS/TS adapter

**Detection globs:** `**/package.json`
**Supporting files:** `README.md` in same directory

### package.json field mapping

| Field | Maps to |
|-------|---------|
| `name`, `description` | `name`, `description` |
| `version` | `version` |
| `main`, `exports`, `module` | `entryPoints` |
| `bin` | `entryPoints` + role hint → `"cli"` |
| `scripts` | `scripts` |
| `dependencies` | `dependencies` + `stack` (framework detection) |
| `devDependencies` | `devDependencies` + `stack` (tool detection) |
| `keywords` | role hints |
| `files` | scope of public API |
| `workspaces` | discover child packages |

### Monorepo awareness

The root `package.json` in a monorepo is the **workspace manager** — it orchestrates child packages but is not itself a deployable unit. The adapter must distinguish it from leaf packages:

```
has "workspaces" field          → role: "workspace-root"
  ↓
expand workspaces globs to discover children
  ↓
each child package.json         → role: inferred normally (cli, lib, app...)
```

A workspace root's `scripts` are aggregate tasks (`build:all`, `test`, `lint`). Its `dependencies` are usually empty — devDependencies holds shared tooling. Its `name` is the repo/org name, not a publishable package name.

The adapter records the workspace root separately and links child paths to it:

```json
{
  "path": ".",
  "role": "workspace-root",
  "name": "skills",
  "workspaces": ["packages/sensei", "packages/web"],
  "children": ["packages/sensei", "packages/web"]
}
```

Children reference their parent:

```json
{
  "path": "packages/sensei",
  "role": "cli",
  "workspaceRoot": "."
}
```

This lets consumers traverse the tree either top-down (from root → children) or bottom-up (from package → its workspace).

### Role inference

```
has "workspaces"                → "workspace-root"  (checked first)
has "bin"                       → "cli"
has "main"/"exports", no "bin"  → "lib"
scripts has "dev" or "start"    → "app"
name contains "ui" or "web"     → "ui"  (override if bin also present → "cli")
```

### Stack inference

Framework and tool names are matched against known lists in dependencies and devDependencies. Output is a flat list: `["typescript", "bun", "vitest", "react", "drizzle-orm"]`. This replaces and improves the current `detectStack` in `reindex.ts`.

---

## Future adapters

Each only needs to implement the same interface. Planned:

| Adapter | Globs | Key source |
|---------|-------|------------|
| Python | `**/pyproject.toml`, `**/setup.py`, `**/requirements.txt` | `[project]` / `[tool.poetry]` sections |
| Go | `**/go.mod` | module name, `require` block |
| Rust | `**/Cargo.toml` | `[package]` name/desc, `[dependencies]` |
| Ruby | `**/Gemfile`, `**/*.gemspec` | gem metadata |

All emit the same `PackageInfo` shape.

---

## Output artifacts

### `.sensei/folder-map.json`

Array of `PackageInfo`. One entry per discovered package boundary.

```json
[
  {
    "path": "packages/sensei",
    "adapter": "npm",
    "name": "@skills/sensei",
    "description": "AI skills toolchain CLI and MCP server",
    "version": "0.1.0",
    "role": "cli",
    "entryPoints": ["dist/cli.js", "dist/index.js"],
    "stack": ["typescript", "bun", "vitest"],
    "scripts": { "build": "bun run build", "test": "bun test" },
    "dependencies": { "@clack/prompts": "^0.9.0", "js-yaml": "^4.1.0" },
    "devDependencies": { "vitest": "^3.0.0" },
    "readme": {
      "title": "sensei",
      "description": "AI skills toolchain for keeping docs and code in sync.",
      "sections": ["Installation", "Commands", "MCP Tools", "Development"],
      "links": [
        { "from": "packages/sensei/README.md", "to": "docs/design/15-package-adapters.md",
          "text": "Package adapter design", "kind": "doc" }
      ]
    },
    "links": [...]
  }
]
```

### Enhanced `.sensei/traceability.json`

Current traceability only captures `doc → [code files]`. With link extraction it becomes a richer graph:

```json
{
  "packages/sensei/README.md": {
    "covers": ["src/cli.ts", "src/index.ts"],
    "linkedDocs": ["docs/design/15-package-adapters.md"],
    "linkedCode": ["src/commands/init.ts"],
    "source": "readme-links"
  },
  "docs/design/15-package-adapters.md": {
    "covers": ["src/tools/reindex.ts"],
    "linkedDocs": [],
    "linkedCode": [],
    "source": "llmspec"
  }
}
```

### Auto-populated sections in `llmspec.yaml`

The llmspec sections below are currently written by hand. With adapters they become auto-generated and kept fresh on every `sensei index`:

```yaml
# auto-generated — do not edit by hand
packages:
  - path: packages/sensei
    role: cli
    name: "@skills/sensei"
    description: "AI skills toolchain CLI and MCP server"
    entryPoints: [dist/cli.js, dist/index.js]
    stack: [typescript, bun, vitest]

stack: [typescript, bun, vitest]          # union across all packages

shortcuts:                                 # merged, namespace-prefixed in monorepos
  build: bun run build
  test: bun test
```

---

## Drift integration

With the link graph in traceability, drift detection gains a new signal:

| What changed | What might be drifted |
|---|---|
| `src/commands/init.ts` | Any doc whose `covers` or `linkedCode` includes it |
| `docs/features/auth.md` | Any README whose `linkedDocs` includes it |
| `package.json` version bump | README "Installation" section (if it mentions a version) |

This closes the loop: design docs that reference implementation files will be flagged if the implementation changes — even when there's no explicit `@covers` annotation.

---

## Implementation plan

1. `src/adapters/types.ts` — `PackageAdapter`, `PackageInfo`, `DocLink` interfaces
2. `src/adapters/npm.ts` — JS/TS adapter (package.json + README)
3. `src/adapters/readme.ts` — shared README parser (used by all adapters)
4. `src/tools/folder-map.ts` — scanner: glob, group by dir, run adapters
5. Wire into `reindexRepo` — run after symbol map, write `folder-map.json`
6. Update `traceability.ts` — merge link graph from folder map
7. Update `llmspec` auto-generation — populate `packages` and `stack` from folder map
8. Update `checkDrift` — use enhanced traceability
