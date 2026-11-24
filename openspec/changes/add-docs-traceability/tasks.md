# Tasks: add-docs-traceability (#246)

TDD: red test first for each item.

- [x] 1. Routing: `rst`/`adoc` to the doc processor; router tests.
      (Adoc `=` and rst underline headings decompose; frontmatter
      skipped so fences never read as headings; title follows.)
- [ ] 2. Edges: `file_refs` resolve to `documents` edges (intra-repo);
      externals named, never dropped; `fn_mentions` resolve to symbols;
      prune on re-index; tests per case.
- [ ] 3. Accounting: docs in map counts with the `docs` role; documented
      shade stays code-only; verification tests.
- [ ] 4. Gates green, commit, push; close #246 with the World note.
