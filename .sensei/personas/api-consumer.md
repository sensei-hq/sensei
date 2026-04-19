---
name: API Consumer
category: persona
description: Developer integrating with sensei MCP or HTTP API
goals:
  - Find the right MCP tool for my task
  - Understand tool parameters without trial and error
  - Get predictable, structured responses
pain_points:
  - Undocumented tool parameters
  - Inconsistent response formats between tools
  - No error schema — can't programmatically handle failures
validates:
  - Is every MCP tool documented with params and return type?
  - Are error responses structured and consistent?
  - Can I discover available tools without reading source code?
---

# API Consumer

A developer building on top of sensei's MCP or HTTP API. They might be building a custom IDE integration, a CI pipeline, or a dashboard.

## Journey

1. Discovers sensei has an API (MCP or HTTP)
2. Looks for tool/endpoint documentation
3. Tries a call — needs to understand params and response shape
4. Handles errors — needs structured error responses
5. Builds a workflow — chains multiple calls together
6. Relies on stable, predictable behaviour

## Questions

Ask these when building or changing anything this persona touches:

1. **Can I discover this tool/endpoint without reading source?** — Is there a list of all tools with descriptions, params, and return types?
2. **Are the params obvious?** — Does the name and type tell me what to pass? Or do I need to look at an example?
3. **Is the response shape predictable?** — Same structure every time — success and error. No surprises.
4. **What happens on error?** — Structured error with a code, message, and suggestion — not a stack trace or empty response.
5. **Will this break when sensei upgrades?** — Are there versioned endpoints? Deprecation warnings? Migration guides?

## What frustrates them

- **Discovery** — no single list of "here are all the tools and what they do." Has to read source code.
- **Inconsistency** — some tools return `{ok: true, data: ...}`, others return raw data. Error shapes vary.
- **Breaking changes** — tool params change between versions with no migration guide.
