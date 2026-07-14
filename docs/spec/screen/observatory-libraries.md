# 庫 · Observatory · Libraries

**Segment:** 03 · Observatory — daily use
**Route:** `/libraries`
**Source mockup:** [`lib/observatory/libraries.jsx`](../../mockups/Sensei/lib/observatory/libraries.jsx) → `LibrariesVariantA`
**App file:** `app/src/routes/(observatory)/libraries/+page.svelte`

## Purpose

The user's library shelf across every project. Answers *"what am
I depending on, what's active, what's drifting, what's worth
wrapping?"* Filterable by ecosystem (npm / crates / pip / …),
by usage tier, by whether it's wrapped, by whether it has drift
open against its wrapper.

Kanji is 庫 — *repository*.

## Data invariants

- `GET /api/libraries` returns rows from `sensei.libraries` +
  usage rollups (see [[pipeline/libraries]]).
- Each row: name, version, ecosystem, source, `usage_count_14d`,
  `wrapped`, `docs_source_kind`, `drift_open_count`.
- Wrap-me candidates surface as a strip above the list —
  same primitive as Insights Now column but library-scoped.
- Add-library action opens a small pane accepting local path /
  GitHub URL / website URL (see [[pipeline/libraries]] ingestion).

## Signals shown

| Element | Value |
|---|---|
| Ecosystem filter chips | npm / crates / pip / go / ruby / … with counts |
| Usage tier chips | High (≥50 14d) / Medium / Low / Unused |
| Wrapped filter | wrapped / unwrapped / any |
| Drift filter | open drift only |
| Add library button | opens the add-library pane |
| Wrap-me strip | up to 3 top candidates with reasons |
| Library row | icon · name · version · ecosystem chip · usage · wrapped chip · drift chip |
| Row expand | shows recent projects using it + docs source + wrap-me action |

## Done gate

- Every library detected by the scanner appears; ecosystem +
  wrapped + drift chips render truthfully.
- Wrap-me strip surfaces up to 3 candidates; each cited
  candidate has `usage_count_14d >= WRAP_MIN` (default 12) AND
  `wrapped = false`.
- Add-library succeeds for all three source shapes (local /
  github / website llms url).
- Clicking a row's docs source opens
  [[screen/observatory-instruments-playground]] with the tool
  focused on `search_lib_docs` and the library pre-filled.
- Drift chip count on the row matches
  [[pipeline/traceability]] counts for that library.

Optional check:
```
curl -s http://localhost:7744/api/libraries \
  | jq '{n_libs: (.libraries | length),
         wrap_me: [.libraries[] | select(.usage_count_14d >= 12 and (.wrapped | not))] | length}'
# expected: n_libs > 0 on Jerry's data; wrap_me is what the strip shows
```

## Wrong gate

- **A library with 200 imports doesn't appear as high-usage.**
  Rollup query missing.
- **Add-library succeeds but no pages ingested.** Silent failure.
- **Wrap-me candidate list is empty even though several
  libraries cross WRAP_MIN.** Threshold logic OR the strip
  reads the wrong table.
- **Drift chip doesn't reflect wrapped-library drift.**
- **Ecosystem chip shows `crates` for an npm library.** Ecosystem
  detection wrong.

## Related

- [[pipeline/libraries]] — data source
- [[pipeline/traceability]] — drift chip
- [[screen/project-libraries]] — project-scoped one-click wrap
- [[screen/observatory-instruments-playground]] — docs query
