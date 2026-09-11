# `sensei.library.json` — the library manifest

How a library tells sensei what it is, what it provides, and where its
documentation lives. One file, published in the repo AND served from the site.

Every rule below is here because something measured went wrong without it. The
evidence is named inline; nothing is speculative.

## 1. Discovery

A library is discoverable from its site at either path, `.well-known` first:

    /.well-known/sensei.library.json     RFC 8615 reserved namespace
    /sensei.library.json                 fallback

**A consumer MUST NOT treat `200` as "the manifest exists."** SPA hosts answer
`200 text/html` for every path. Accept only when all three hold:

1. `content-type` is JSON,
2. the body parses,
3. `library` is present.

MEASURED: a library's published homepage pointed at an unrelated application
whose SPA returned `200 text/html` for `/llms.txt`. 1,206 bytes of a stranger's
page were parsed and stored as that library's documentation. Every layer
reported success.

## 2. Shape

```jsonc
{
  // ── identity ──────────────────────────────────────────────────────────
  "library":   "rokkit",              // REQUIRED. The name capabilities hang off.
  "version":   ">=1.3",               // The RANGE these capabilities apply to.
  "documents": "1.4.1",               // REQUIRED. The concrete release the
                                      // published artifacts describe.
  "ecosystem": "npm",                 // npm | cargo | pypi | go | …

  // ── where the source is ───────────────────────────────────────────────
  "repo": "https://github.com/jerrythomas/rokkit",
  "ref":  "v1.4.1",                   // A TAG. Not a branch.
  "site": "https://rokkit.sensei-hq.com",

  // ── what it publishes ─────────────────────────────────────────────────
  "packages": ["@rokkit/ui", "@rokkit/core", "…"],

  // ── where the docs corpus is ──────────────────────────────────────────
  "llms": {
    "index": "/llms/index.txt",       // REQUIRED for docs discovery.
    "path":  "docs/llms",             // repo-relative, for the git/local routes
    "focus": "component docs, guides, package APIs"
  },

  "skills":  [{ "name": "…", "focus": "…", "path": "…", "url": "…" }],
  "agents":  [{ "name": "…", "focus": "…", "path": "…", "url": "…" }],
  "install": { "skills": "rokkit skills add <name>",
               "agents": "rokkit agents add <name>" }
}
```

## 3. The fields that are currently wrong or missing, and why they matter

### 3.1 `documents` — a range is not a release

All three reference libraries declare `version` as a RANGE (`>=1.3`, `>=1.0`,
`>=0.10`). A range says which releases the capabilities APPLY TO. It does not
say which release the published artifacts DESCRIBE, and those are different
questions.

Without it, documentation fetched from a site is stored under the literal
version `latest` with nothing to compare against, so:

- "are these docs for the version I depend on?" is unanswerable, and
- "is our copy of `latest` still current?" is unanswerable.

A consumer that cannot answer the first has to either serve possibly-wrong docs
silently, or refuse to serve any. Both are bad; the label that makes a
version-mismatched answer *useful* needs a concrete version to name.

Keep `version` — the applies-to range is real information. Add `documents`.

### 3.2 `ref` — a branch is a moving target

`"branch": "main"` names whatever main holds today, so docs fetched from it are
not reproducible and cannot be matched to a release.

A TAG is. MEASURED: fetching `docs/llms` at `v1.4.1`, `v1.1.3` and `v0.12.6`
returned 94, 15 and 43 pages respectively — each matching that library's local
walk exactly. The GitHub contents API takes a tag anywhere it takes a branch,
so this costs nothing.

Prefer `ref` with a tag. `branch` is accepted as a fallback and treated as a
ref, but it cannot be version-matched.

### 3.3 `llms.index` — otherwise the corpus is guesswork

A library that omits this cannot have its documentation discovered; a consumer
is reduced to guessing conventional paths.

MEASURED: guessing `/llms.txt` produced a 404 for one library (its corpus is at
`/llms/index.txt`), and a stranger's HTML for another (its corpus is at
`/llms/llms.txt`). Both libraries declared the correct path in their manifest.
Reading it first would have avoided both.

### 3.4 `packages` — grouping must be declared, never inferred

A library publishing under names other than its own (`rokkit` →
`@rokkit/ui`, `@rokkit/core`, …) needs to say so, or a dependency on
`@rokkit/ui` cannot reach `rokkit`'s skills and docs.

A prefix rule is NOT an acceptable substitute: `@types/node` belongs to no
"types" library, and plenty of libraries publish packages named nothing like
themselves. The library is the only party that knows its own membership.

Consumers may FALL BACK to workspace members (`package.json` `workspaces`,
`Cargo.toml` `[workspace] members`), which is equally declared, just stated
elsewhere — but that misses any package not laid out as a workspace member, and
it wrongly includes private ones unless filtered.

### 3.5 `ecosystem` — identity is `(ecosystem, name)`

A library is keyed on the pair. Omitting it forces a consumer to infer the
ecosystem from whatever manifest sits beside it, and a wrong guess mints a
second identity for the same library — after which the grouping attaches to the
wrong row.

## 4. Freshness

Serve the manifest and the corpus with `ETag` (or `Last-Modified`). A
conditional `GET` then answers "unchanged" in one round trip with no body.

No feed is needed. A changelog feed answers a different question ("when did it
change") and would be a second source of truth to keep aligned with the
manifest.

## 5. Serving

- `content-type: application/json` for the manifest.
- Text artifacts (`.txt`, `.md`) MUST NOT be served as `text/html`. A consumer
  that asked for a text document and received HTML has been told "no" in a way
  that looks like "yes", and must reject it.
- Repo copy and served copy should be the same file. All three reference
  libraries already satisfy this; the divergence risk is a build step that
  rewrites one.
