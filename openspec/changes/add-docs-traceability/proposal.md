# Change: Docs in the graph — traceability edges and full doc routing (#246)

## Why
#246 asked for documentation as first-class graph citizens. Since it was
filed, two things changed: D5b landed markdown sections as `section` nodes,
and M17 retired the World screen the issue's done-condition names. What
remains is the connective tissue the issue describes as "the feature behind
the ring": doc→source links are extracted and dropped
(`create_traceability_edges` is a `dead_code` TODO), and `rst`/`adoc` never
reach the doc processor. Until the links resolve, "what documents this
module" is unanswerable and docs sit beside the graph instead of in it.

## What Changes
- **Routing.** `rst` and `adoc` route to the doc processor, completing the
  `file_role` convention (`md | mdx | rst | adoc | txt`, decided with the
  user 2026-11-24).
- **Traceability edges.** Doc links resolve against the checkout's indexed
  files through the D2 path: an intra-repo link becomes a `references` edge
  (the edge-kind contract's kind for doc refs) from the doc's node to the
  file's; an external URL becomes a named-unresolved `references` edge,
  never dropped. Link syntaxes grow past backticks (markdown links, rst and
  adoc targets) and `../` resolves against the doc's own dir, contained to
  the repo. `fn_mentions` keep the sole-definition rule.
- **Map accounting.** Docs files and section nodes flow into the Code map's
  counts with the `docs` role; the documented-share shade stays code-only
  (docstring-based, per code declaration — the decided middle path).

## Non-goals
- The World ring: the World screen was retired (M17); there is no ring to
  add. #246 closes on the graph work with that note.
- Heading semantics: D5b's section nodes (heading-path identity, sibling
  disambiguation, prune on re-index) stand as decided.
- Map visibility for docs: needs fqns, which needs the docs language
  adapter (follow-up issue). The middle-path ruling (docs role counts,
  code-only shade) is recorded there.
