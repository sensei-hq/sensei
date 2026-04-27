# Project Context — sensei docs

> Design documentation, database schema, and project configuration.
> This repo has no runnable code — daemon and app live in separate repos.

## What's here

- `database/` — DDL schema (source of truth for data model discussions, daemon owns runtime migrations)
- `docs/` — ideas, mockups, journeys, design docs, analysis, blueprints
- `.sensei/` — mindsets, personas, project rules
- `config/` — shared config templates

## Related repos (in workspace)

- `../daemon/` — Rust crates + database DDL (runtime)
- `../app/` — Desktop Tauri + SvelteKit app
- `../gateway/` — LLM routing library
- `../marketplace/` — Skills, commands, agents

## Working with this repo

This is a design repo. Tasks here are about:
- Writing/updating ideas, journeys, and design docs
- Updating mockups and the design summary
- Gap analysis between mockups, journeys, and database DDL
- Updating database DDL (then sync changes to `../daemon/database/`)

## Database DDL sync

The `database/` folder here is a design-time reference. The daemon repo has its own copy that it uses for migrations. When DDL changes are made:
1. Edit in whichever repo you're working in
2. Copy the change to the other repo
3. Daemon is the source of truth for runtime schema
