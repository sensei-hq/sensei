---
name: Pattern Knowledge — Beyond Codebase Detection
description: Library-specific patterns, industry patterns, and architectural patterns — knowledge the AI needs beyond what the indexer detects from code
date: 2026-04-17
status: idea
related: 15-pattern-store.md, 08-codebase-intelligence.md, 09-library-intelligence.md
---

# Pattern Knowledge

## Problem

Idea 15 (Pattern Store) handles patterns detected FROM the codebase. But the AI also needs pattern knowledge that doesn't exist in the code yet — library usage patterns, industry best practices, and architectural patterns. Without this knowledge, the AI makes suboptimal choices and the user has to correct it.

### Three real scenarios

**1. Library-specific patterns (rokkit)**

Rokkit data-driven components follow conventions: `Props` interface with `items`/`options`/`fields`, internal `ProxyTree`/`ProxyItem` usage, `Navigable`/`Navigator` actions, `data-attribute` props for components, `data-index` for items. Creating a new component means following this pattern — but the AI doesn't know about it unless pointed to an example. Even then, it may misunderstand.

**2. Industry patterns (patterns.dev)**

Web application patterns — rendering patterns (SSR, SSG, ISR), design patterns (compound components, hooks pattern, provider pattern), performance patterns (code splitting, tree shaking). These are documented externally and applicable to many projects. The AI's training data includes some, but it doesn't consistently apply the right pattern for the right situation.

**3. Architectural patterns (i18n, data separation, progressive enhancement)**

Cross-cutting patterns that affect how code should be structured:
- **i18n-ready**: strings in messages collection, not hardcoded → adding a language is just adding strings
- **SSR data separation**: page focuses on UI, loader fetches data → mockup can be enhanced with API calls without rewrite
- **Progressive enhancement**: static → data → interactive in small incremental steps

These overlap with what `working-smarter` tried to enforce, but they need to be documented as chooseable options, not hardcoded rules.

---

## Pattern Knowledge Sources

| Source | How it enters sensei | Examples |
|--------|---------------------|----------|
| **Codebase** (detected) | Indexer auto-detection (idea 08, 15) | Adapter pattern, task worker, module structure |
| **Library** (from docs) | Library intelligence (idea 09) — extract usage patterns from indexed library docs | Rokkit component conventions, kavach auth flows |
| **Industry** (curated) | Pattern registry — import from curated sources like patterns.dev | SSR/SSG patterns, compound components, observer |
| **Architectural** (configured) | User/project config + guardrails — user declares which patterns apply | i18n approach, data separation strategy, testing approach |
| **Derived** (from usage) | AI observes which patterns the user has used before in this project | "You've used SSR data separation in all other pages — use it here too" |

---

## Library Pattern Extraction (rokkit example)

Library docs alone aren't enough — the AI needs to understand **usage patterns** specific to the library. For rokkit:

### What the AI needs to know

```yaml
library: rokkit
patterns:
  - name: data-driven-component
    description: All interactive components accept data via a consistent Props interface
    interface:
      props:
        items: "T[]"                    # or options, fields — the data array
        snippets: "Record<string, Snippet>"  # render customization
        selected: "T | T[]"            # selection state
    internals:
      - ProxyTree wraps the data for navigation
      - ProxyItem represents each navigable item
      - Navigable mixin handles keyboard/focus
      - Navigator coordinates selection across items
    html_conventions:
      - "data-component" attribute on root element
      - "data-index" attribute on each item element
      - "data-selected" attribute for selected state
    creating_new:
      1. Define Props interface extending BaseComponentProps
      2. Create ProxyTree from items in component init
      3. Use Navigable for keyboard handling
      4. Apply data-attributes on elements
      5. Register in component index
    reference_implementations:
      - "List (simplest)"
      - "Tabs (with panels)"
      - "Tree (nested, recursive)"
```

### How to extract this

1. **From library docs**: If rokkit has llms.txt or component docs, the pattern can be extracted during library indexing (idea 09). The `add_library` flow could include a "extract usage patterns" step.

2. **From codebase usage**: The indexer sees how rokkit components are used in the project — which props are passed, which patterns repeat. Cross-reference with library API to derive conventions.

3. **From user declaration**: User runs `/sensei:pattern-extract` and points to a rokkit component as the reference. AI extracts the pattern, asks clarifying questions, documents it.

4. **From library author**: The library itself could ship a `patterns.yaml` or include patterns in its llms.txt. This is the ideal — patterns documented at the source.

---

## Industry Pattern Registry

Curated pattern collections like [patterns.dev](https://www.patterns.dev) contain well-documented patterns for web applications. Sensei should be able to import and reference these.

### How it works

```yaml
# .sensei/pattern-registries.yaml
registries:
  - name: patterns.dev
    url: https://www.patterns.dev
    indexed: true
    categories:
      - rendering       # SSR, SSG, ISR, streaming
      - design          # compound components, hooks, provider
      - performance     # code splitting, prefetching, tree shaking
```

**Indexing external patterns:**
1. Fetch and parse the pattern source (via library intelligence pipeline — idea 09)
2. Store as pattern nodes in the graph with `source: "patterns.dev"` tag
3. Each pattern includes: name, category, problem it solves, when to use, when NOT to use, tradeoffs
4. MCP tool `match_pattern(description)` searches both codebase patterns AND registry patterns

**How the AI uses it:**

When the AI encounters a design decision, it checks both sources:

```
User: "Build a settings page with tabs"

AI thinking:
1. get_patterns("tabs") → codebase: "rokkit data-driven-component" (4 instances)
2. match_pattern("tabbed interface with data loading") →
   - codebase: rokkit Tabs component pattern
   - registry: "Compound Components" pattern (patterns.dev)
   - registry: "Provider Pattern" for shared state (patterns.dev)
3. Check project usage: "Previous pages use SSR data separation with rokkit"

AI response:
"I found 3 applicable patterns:
 1. **Rokkit data-driven Tabs** — your project uses rokkit Tabs in 2 other places.
    I'd follow the same Props/ProxyTree/Navigable conventions.
 2. **SSR data separation** — your other pages load data in server loader,
    render in component. I'll do the same here.
 3. **i18n-ready** — your project uses message collections. I'll use message
    keys for all strings.

 Should I proceed with all three, or is this case different?"
```

---

## Architectural Pattern Options

Some patterns are **choices** — the user picks which approach to use, and the AI follows it consistently. These should be documented as options with tradeoffs, not hardcoded rules.

### Pattern option: String handling

| Option | Description | When to use | Tradeoff |
|--------|-------------|-------------|----------|
| **Hardcoded** | Strings inline in components | Prototype, single-language app | Fast to build, impossible to translate |
| **Messages collection** | Strings in a messages file, referenced by key | Multi-language app, or planning for it | Slightly more setup, but adding a language = adding a file |
| **Runtime i18n** | Full i18n framework (i18next, paraglide) | Large app, dynamic language switching | Most flexible, most setup |

### Pattern option: Data loading

| Option | Description | When to use | Tradeoff |
|--------|-------------|-------------|----------|
| **Inline data** | Data hardcoded in component | Mockup, static demo | Fastest to build, requires rewrite for real data |
| **SSR loader separation** | Page = UI, loader = data fetching | Production app with server rendering | Clean separation, mockup-to-production is just swapping the loader |
| **Client-side fetch** | useEffect/onMount API calls | SPA, real-time data | More client JS, loading states needed |
| **Reactive streams** | RxJS/observable-based data flow | Complex real-time apps, event-driven UIs | Most powerful, steepest learning curve |

### How options work in practice

1. **First time**: AI presents options with tradeoffs, user picks. Decision is recorded:
   ```
   AI: "For string handling, I see 3 options: [hardcoded, messages, runtime i18n].
        Your project already uses messages collections in other pages.
        Should I follow that pattern?"
   User: "Yes, use messages"
   → AI adds to guardrails: "Use messages collection for all UI strings"
   → Next time: AI doesn't ask, just uses messages
   ```

2. **Subsequent times**: AI checks guardrails + project usage → follows established pattern automatically. If the task seems different (e.g., a throwaway mockup), it asks: "Your project uses messages for strings, but this is a mockup — hardcoded OK, or messages?"

3. **New project**: AI has no project history. Presents options based on stack detection + industry patterns. User picks. Pattern established.

---

## Derived Patterns (from usage observation)

The most powerful source: **what the user has already done in this project**. The indexer can detect:

| Signal | What it reveals |
|--------|-----------------|
| All pages load data in server loaders | SSR data separation is the established pattern |
| All strings go through `$t()` calls | i18n is active — all new strings must use it |
| All API routes follow RESTful naming | REST conventions are the established pattern |
| All components use rokkit's Props interface | rokkit data-driven pattern is the standard |
| Tests use `describe/it/expect` with fixtures in `__fixtures__/` | Test structure pattern |

These derived patterns are **implicit guardrails** — the AI should follow them without being told. The MCP tool `get_patterns()` should surface both explicitly documented patterns AND derived patterns from usage analysis.

---

## MCP tools (additions to idea 15)

| Tool | Purpose |
|------|---------|
| `match_pattern(description, sources?)` | Find applicable patterns from ALL sources — codebase, library, registry, derived. Returns ranked matches with tradeoffs. |
| `get_pattern_options(category)` | List architectural options for a category (string handling, data loading, etc.) with tradeoffs. |
| `get_project_conventions()` | Return derived patterns from project usage — what the project consistently does. |

---

## Impact on commands

| Command | How pattern knowledge helps |
|---------|---------------------------|
| `/sensei:build` | Locate step: `match_pattern()` checks ALL sources. AI presents options if new territory, follows convention if established. |
| `/sensei:review` | Checks for pattern violations across ALL sources — not just codebase patterns but also library conventions, established choices. |
| `/sensei:brainstorm` | When discussing implementation approach, AI presents pattern options with tradeoffs instead of picking one silently. |
| `/sensei:patterns` | Shows full pattern catalog — codebase, library, registry, derived. User can explore and configure. |
| `/sensei:rules` | Architectural choices (i18n, data loading) are recorded as guardrails once the user picks an option. |

---

## Open questions

| # | Question |
|---|----------|
| 1 | Should library authors ship `patterns.yaml` as part of their package? Could sensei define a standard for this? |
| 2 | How do we index patterns.dev (or similar) — web scrape, or ask the community to contribute structured data? |
| 3 | How many "architectural option" categories exist? String handling, data loading, state management, routing, testing, error handling — is there a finite list? |
| 4 | Should derived patterns auto-promote to guardrails after N consistent usages? Or always require user confirmation? |
| 5 | How do we handle pattern conflicts? (e.g., rokkit pattern says X, patterns.dev says Y) |
