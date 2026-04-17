---
id: doc-tools
type: design
implements:
  - feature: documentation
    items: [doc-guide, find-doc, scaffold, external-doc-ref, doctor-single-doc, doctor-directory, template-auto-detection, doctor-rules]
---

# Doc Tools

## Overview

Doc Tools covers four capabilities: the `doc-guide` skill that encodes naming conventions and the traceability workflow, the `find_doc` fallback that locates existing docs without MCP, the `sensei doc new` scaffold that creates matched feature+design pairs, and `sensei doctor` which reformats existing docs to canonical templates without losing content. The doc-guide skill is the primary interface for agents; the CLI commands handle developer-initiated workflows.

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| accuracy | Doc doctor must preserve 100% of existing content — no silent drops |
| usability | Template auto-detection must require zero user input for standard paths |
| maintainability | Templates must be editable plain markdown |

---

## find_doc — Doc Discovery

`find_doc` locates an existing doc by topic before the agent creates a new one. It prevents duplicates.

### With MCP

```typescript
find_doc(query: string): DocMatch | null
// Returns: { path, id, type, summary } or null
```

### Without MCP (fallback)

```
1. Glob docs/**/*.md          ← collect all doc paths
2. For each file: Grep for frontmatter `id:` field and first paragraph
3. Match query against id values and first-paragraph text
4. Return best match path + id, or null if no match
```

### Rules

- Always call `find_doc` before creating any new doc
- If found: update the existing doc, do NOT create a duplicate
- If not found: proceed to create using the appropriate template

---

## Scaffold — sensei doc new

`sensei doc new "<Name>"` creates a matched feature+design doc pair from a single command.

### Flow

```
Step 1: Check docs/features/ for existing doc matching name (exit with error if found)
Step 2: Find next NN for features: ls docs/features/ | sort | tail -1 → increment
Step 3: Find next NN for design: ls docs/design/ | sort | tail -1 → increment
Step 4: Create docs/features/NN-<name>.md from skills/doc-guide/feature-template.md
Step 5: Create docs/design/NN-<name>.md from skills/doc-guide/design-template.md
Step 6: Add entries to docs/traceability.yaml under features: and design:
Step 7: Update docs/features/README.md — add module to list
Step 8: Print: "Created docs/features/NN-<name>.md and docs/design/NN-<name>.md"
```

### CLI Interface

```
sensei doc new "<Name>" [--feature-only] [--design-only]

Arguments:
  Name              PascalCase or hyphen-case module name

Options:
  --feature-only    Create only the feature doc
  --design-only     Create only the design doc
```

### Error Handling

```
Name already exists:   "docs/features/NN-<name>.md already exists. Use sensei doctor to update it."
Traceability missing:  Create docs/traceability.yaml if it doesn't exist, then add entries
```

---

## External Doc Reference — fetch_doc_ref

`fetch_doc_ref` fetches external API documentation and caches it in `.sensei/doc-refs/` with a 7-day TTL so repeated calls within the same week skip the network.

### Cache Layout

```
.sensei/doc-refs/
  <slug>.md         ← cached doc content (markdown)
  <slug>.meta.json  ← { url, fetchedAt, ttlDays, tags }
  index.json        ← lightweight index for search_doc_refs
```

### Flow

```
Step 1: Check .sensei/doc-refs/<slug>.md
  → If exists and age < ttlDays: return cached content
  → If exists and stale: re-fetch
  → If not exists: fetch
Step 2: Fetch URL, convert to markdown (strip nav/footer)
Step 3: Write <slug>.md + <slug>.meta.json
Step 4: Update index.json with { slug, url, summary, tags, fetchedAt }
Step 5: Return content
```

### API / Tool Contracts

```typescript
fetch_doc_ref(query: string): DocRefResult
// query: "anthropic messages api" — resolves to known URL or searches for one
// Returns: { content, url, cached: boolean, fetchedAt }

search_doc_refs(query: string): DocRefMatch[]
// Searches across all cached ref index entries
// Returns: [{ slug, url, matchedSection, relevance }]
```

### Error Handling

```
URL not found:        "Could not resolve doc ref for '<query>'. Provide a URL directly."
Fetch failure:        "Failed to fetch '<url>'. Cached version returned if available."
No cached version:    Error with fetch failure message, no fallback
```

---

## Template Detection

Templates live in `docs/templates/`. Detection is path-based:

| Path pattern | Template used |
|---|---|
| `docs/design/**` | `docs/templates/design.md` |
| `docs/features/**` | `docs/templates/feature.md` |
| `docs/requirements/**` | `docs/templates/feature.md` |
| `docs/plans/**` | Skipped — plans are implementation artifacts |
| Anything else | Interactive prompt: select template |

Detection order: exact path match → directory pattern → interactive fallback.

---

## Prompt Structure

The output of `sensei doctor <file>` is a prompt sent to Claude via stdin or printed for the developer to paste:

```
Doctor the following document to match the canonical template.

## Template

[full contents of docs/templates/design.md or feature.md]

## Existing Document

[full contents of the target file]

## Rules

1. Preserve ALL existing information — restructure only, do not summarise away details
2. Add missing template sections with placeholder text: "TODO: [section description]"
3. Place any content that doesn't fit the template under "## Additional Notes"
4. Do not invent information — only reorganise what exists
5. Keep all code blocks, tables, and examples intact
6. Output the complete doctorted document only — no preamble or explanation
```

---

## CLI Interface

```
sensei doctor <path> [--dry-run] [--template <path>]

Arguments:
  path              File or directory to doctor

Options:
  --dry-run         Print the prompt without running Claude
  --template <path> Override auto-detected template
```

**Single file:** builds prompt, passes to Claude, writes result back to the file.

**Directory:** iterates files, presents each prompt to Claude in sequence with a confirm step between each:
```
Doctoring docs/design/03-auth.md... [done]
Doctoring docs/design/04-payments.md... approve? (y/n/skip)
```

---

## Tool Contract

### `sensei doctor <file>`

```typescript
async function doctor(filePath: string, options: {
  dryRun?: boolean;
  template?: string;
}): Promise<void>

// 1. Detect template from path (or use options.template)
// 2. Read template content
// 3. Read existing file content
// 4. Build prompt string
// 5. If dryRun: print prompt and exit
// 6. Pass prompt to Claude (via SDK or MCP tool)
// 7. Write result back to file
// 8. Print: "Doctored docs/design/03-auth.md"
```

---

## Doc Doctorter Skill

The `doc-doctor` skill teaches Claude the doctorting protocol for in-session use (without CLI):

```
name: doc-doctor
trigger: when asked to doctor, restructure, or migrate docs to match a template
```

**In-session workflow:**
```
1. call: get_file_context("docs/templates/design.md", "L3")   ← load template
2. call: get_file_context("docs/design/03-auth.md", "L3")     ← load existing doc
3. Rewrite the doc following the rules above
4. Write the doctorted content back to the file
5. call: checkpoint("Doctored docs/design/03-auth.md")
```

This mirrors what the CLI does but works directly in a Claude session without the `sensei doctor` command.

---

## Testing Strategy

```
Unit: src/commands/doctor.spec.ts
  - template detection from various paths
  - prompt structure (contains template + existing content + rules)
  - dry-run outputs prompt, does not write file
  - directory batch iterates all matching files

E2E: e2e/doctor.e2e.ts
  - single file doctor round-trip
  - directory batch with skip

Unit: src/commands/doc-new.spec.ts
  - scaffold creates matching NN pair
  - errors if name already exists
  - updates traceability.yaml and README

Unit: src/doc-refs/fetch-doc-ref.spec.ts
  - returns cached version when fresh
  - re-fetches when stale
  - search_doc_refs finds matching entries
```
