# Codebase Extractor Guide

What to extract when indexing a repo, and how.

## File Map

- Run directory tree limited to 3 levels deep
- Identify entry points: files named `index`, `main`, `app`, `server`, `router`, `config`
- Note any existing `CLAUDE.md`, `.cursorrules`, `llms.txt`, `.llmspec.yaml`

## Tech Stack

- Read `package.json`, `pyproject.toml`, `Cargo.toml`, `go.mod`, `pom.xml`
- Extract: language, framework, major deps (ignore dev tooling like eslint, prettier)
- Note package manager: npm/yarn/pnpm/bun/pip/cargo/etc.

## Code Patterns

Read 3-5 representative files. Look for:
- Naming conventions (camelCase vs snake_case, file naming)
- File organisation (feature folders vs type folders)
- Common idioms (error handling style, async patterns, DI approach)
- Test file location and naming

## Symbols (store at all 4 levels)

For each module/file:
- L0: extract all exported function/class signatures
- L1: extract IO patterns (parameter types + return type)
- L2: summarise logic in 3-7 bullet steps
- L3: full source (reference path only, don't copy to index)

Tools to use: Grep for `export`, `def `, `func `, `pub fn`

## Dev Shortcuts

Read: `package.json scripts`, `Makefile`, `justfile`, `taskfile.yaml`, `scripts/` directory
Extract: dev, test, build, lint, index commands

## Documentation Layers

- Design/feature: `docs/`, `ADR/`, `docs/plans/`, `docs/decisions/`
- Public: `README.md`, `docs/guides/`, `CHANGELOG.md`, `docs/api/`
- Code: source root (for drift detection, not content indexing)

Record file paths + last-modified timestamps for drift detection.

## MCP Configs

Look for: `.mcp.json`, `mcp.config.json`, `.claude/mcp.json`, any MCP server references in README.

## What NOT to Index

- `node_modules/`, `dist/`, `.git/`, `coverage/`, `.cache/`
- Lock files (`package-lock.json`, `yarn.lock`, `bun.lock`)
- Generated files (`.d.ts` files, compiled output)
- Binary files, images, fonts
