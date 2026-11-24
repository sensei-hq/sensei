# Tasks: add-docs-traceability (#246)

TDD: red test first for each item.

- [x] 1. Routing: `rst`/`adoc` to the doc processor; router tests.
      (Adoc `=` and rst underline headings decompose; frontmatter
      skipped so fences never read as headings; title follows.)
- [x] 2. Edges: doc links resolve to `references` edges (intra-repo);
      externals named-unresolved, never dropped; `fn_mentions` keep the
      sole-definition rule; prune rides the D2/D3 reconcile; tests per
      case. (Also fixed: file_ref arm was dead — `kind='file'` is never
      minted — now resolves the file's sole module node.)
- [x] 3. Language slugs for `rst`/`adoc`; map visibility deferred to
      the docs-adapter follow-up (doc nodes carry no fqn; decided with
      the user 2026-11-24).
- [ ] 4. Gates green, commit, push; close #246 with the World note.
