# Setup

The setup wizard runs once after bootstrap completes on a fresh install. It walks you through configuring Sensei for your workspace -- scan roots, projects, libraries, inference, and assistant registration. The wizard can be re-entered from Settings at any time, starting from any step.

## Overview

Setup is a 10-step linear sequence with back navigation and a progress indicator. Each step commits its configuration before advancing. You can skip optional steps and return to them later through Settings.

| Step | Name | Kanji | Purpose |
|------|------|-------|---------|
| 1 | Welcome | 礼 | What Sensei is, what to expect |
| 2 | Preferences | 名 | Coding style, communication, sharing |
| 3 | Scan Roots | 庵 | Pick folders to watch |
| 4 | Scan | 観 | Live indexing progress |
| 5 | Projects | 組 | Group and rename discovered projects |
| 6 | Libraries | 書 | Shared and third-party library docs |
| 7 | Instruments | 器 | MCP tool registry per assistant |
| 8 | Inference | 想 | Local and external model configuration |
| 9 | Assignments | 任 | Map models to reasoning roles |
| 10 | Done | 入 | Summary, enter the observatory |

## Step 1: Welcome

Sensei is a teacher, not a linter. It observes your AI-assisted coding sessions -- prompts, corrections, outcomes -- learns what works for your codebase, and begins to surface patterns, prevent repeated mistakes, and improve your first-try rate.

This step sets expectations. You will see three pillars: observe, teach, local. Nothing leaves your machine. Setup takes roughly four minutes.

Click "Begin setup" to proceed.

## Step 2: Preferences

Set your display name (derived from your home directory by default) and configure how Sensei behaves:

- **Communication style** -- correction aggressiveness, digest cadence (daily or weekly), nudge on FTR regression
- **Sharing** -- whether to contribute anonymized learnings, review before sharing, download collective insights
- **Privacy** -- opt-in telemetry, welcome screen toggle

These preferences apply globally and can be overridden per project later. Corrections to your coding style and communication preferences are also learned over time from observed sessions -- Sensei picks up on things like naming conventions, indentation choices, and whether you prefer terse responses.

## Step 3: Scan Roots

Add root directories you want Sensei to watch. Each root is scanned recursively for git repositories and organized into projects. Depth is typically 1-2 levels for plain folders and any depth for git repositories.

Add roots via a folder picker or by typing a path. Remove any root from the list. At least one root is required before continuing.

Sensei needs to know where your code lives before it can discover projects and build its index.

## Step 4: Scan

The scan runs automatically when you advance from Step 3. A live event stream shows discovery in real time:

- **Stats bar** -- roots scanned, repositories discovered, files queued, files processed
- **Left panel** -- project cards materialize as repositories are found, each showing per-folder progress bars, detected stack, and file counts
- **Right panel** -- scrolling activity log with timestamped entries (discovery, queuing, processing, completion)

No input is needed. The live stream prevents the feeling of a frozen UI during what may be a long operation. A completion banner appears when indexing finishes.

## Step 5: Projects

Sensei auto-groups discovered repositories into projects based on directory proximity, shared configuration, and monorepo markers. A project is one or more repositories that form a logical product.

For each project you can:

- Rename it or set a goal and client
- Split a multi-repo group into separate projects
- Merge projects together
- Move individual repositories between projects
- Assign a role per repository (backend, frontend, library, docs, infrastructure)

Correct grouping matters. Sensei's analytics are project-scoped -- a monorepo with three services should be one project, not three.

## Step 6: Libraries

Libraries that do not have their own MCP server are wrapped by Sensei -- it indexes their documentation and exposes search and usage tools through its own MCP interface.

This step shows libraries detected from your dependency manifests (Cargo.toml, package.json, and similar). For each library you see the name, version, language, and doc-indexing status.

Toggle which libraries to index. Import additional libraries by URL or documentation source. You control which libraries are worth the indexing cost.

## Step 7: Instruments

Instruments are external MCP servers -- tools and services that have their own MCP interface (database, deployment, monitoring, design). Sensei recommends instruments based on your detected stack.

For each instrument you see the name, publisher, verification status, tool count, and whether it is recommended for your stack. Stack-based recommendations are pre-checked. Toggle which instruments to install.

This is distinct from Step 6. Libraries are wrapped by Sensei; instruments bring their own tools.

## Step 8: Inference

Sensei detects your hardware (chip, RAM, GPU) and recommends an appropriate model tier.

**Local models** -- listed via Ollama with disk size and purpose. Pre-checked based on hardware capacity. Models can be pulled in the background while you continue setup. Total resource impact (disk and RAM) is displayed.

**External providers** -- API keys are auto-detected from environment variables (ANTHROPIC_API_KEY, OPENAI_API_KEY, and similar). Enter keys manually for providers that were not auto-detected.

External models enable richer reasoning (the multi-model consensus panel), but local models work without them. You can skip this step entirely and configure inference later in Settings.

**Budget limits** -- set monthly cost caps and per-task token limits to stay in control of spending.

## Step 9: Assignments

Five reasoning roles power Sensei's insights engine: inference, consolidation, embedding, voice, and fallback. Each role needs one or more models assigned to it.

Build an ordered priority list per role. The first entry is the primary model; the rest are fallbacks. Drag to reorder. Each model shows its GPU requirement and latency estimate so you can make informed tradeoffs.

Task-to-model mapping happens here -- you decide which models handle which reasoning tasks based on your hardware and budget.

## Step 10: Done

A summary of everything configured: project count, repository count, library count, instrument count, registered assistants, and inference status.

Sensei will watch your sessions quietly for the first few days, building a baseline. Then it begins to teach -- surfacing patterns, flagging repeated mistakes, and recommending improvements.

Click "Enter the observatory" to finish setup and land on your daily view.

## Re-entering setup

Every step is revisitable from Settings at any time:

- **Settings > Folders** -- add a root, trigger a re-scan, see new projects appear
- **Settings > Projects** -- split, merge, move repositories, change roles
- **Settings > Libraries / Instruments** -- toggle indexing, add new sources
- **Settings > Inference / Assignments** -- reconfigure models, update API keys
- **Re-run setup wizard** -- starts fresh from Step 2 (Welcome is skipped on re-entry)

## Reference

- Mockups: `mockups/lib/setup-wizard.jsx`, `mockups/lib/wiz-inference.jsx`, `mockups/lib/wiz-assignments.jsx`
- Design: `design/01-app.md`
