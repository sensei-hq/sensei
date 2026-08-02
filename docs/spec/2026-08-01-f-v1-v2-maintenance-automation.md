# Part F v1/v2 — Scheduled Library-Maintenance Automation

> Status: DESIGN ONLY (blueprint). Builds on F v0 (commit `8e78c344`): the pure policy core
> (`crates/senseid/src/libraries/version.rs`), version resolution
> (`crates/senseid/src/libraries/registry.rs`), and the daily detect+notify tick
> (`crates/senseid/src/tasks/library_update_scheduler.rs`). v0 added ZERO DDL; v1/v2 hold that bar.
>
> Produced by a skeptic-verified ultracode design workflow (18 agents: 5 ground · 3 design ·
> 9 adversarial verify · 1 synthesis). Overall skeptic verdict: **sound** — every HIGH issue folded
> into the design below. Extends [[spec/pipeline/library-intelligence]] and
> [[spec/2026-07-31-sensei-evolution]] Part F.

## Guiding invariants (non-negotiable)

1. **Never fabricate on a failure path.** A miss is `None`/skip+log, never a plausible default. An
   "applied" claim is only ever written after a *confirmed* refresh.
2. **Never auto-change application/dependency code.** Auto-apply is limited to sensei's OWN
   artifacts — re-ingesting doc pages and refreshing manifest-declared skills/agents via
   `index_library`. `referenced_libraries.version_used` and every manifest/lockfile stay read-only.
3. **DRY.** Reuse `update_action` (unchanged), `index_library` (the apply primitive),
   `create_recommendation_full` (the only surface), the props-cache idiom, and the sibling
   schedulers' concrete `Arc<TaskQueue>` injection. No new channel, no new abstraction that
   duplicates existing infra.
4. **Minimize DDL** — target zero (achieved; see DDL section).
5. **Pure, testable policy** — mirror `version.rs`.

---

## 0. Shared prerequisites (fix once; v1 and v2 both depend on these)

The v1/v2 apply path routes through `index_library`, and the daily tick has no queue handle. Three
correctness fixes and one plumbing change are prerequisites — **they gate the apply wiring and must
land with it, not after.**

### 0a. `index_library` must resolve the target BY lib_id (correctness — HIGH)

**Problem.** The enqueue recipe passes the library UUID as `folder_path`
(`Task::new(IndexLibrary, &lib_id.to_string(), name)`, `mcp.rs:184-189`), but `index_library`
**ignores `task.folder_path`** and reads only `lib_name = &task.path` + `task.url`
(`libraries.rs:143-144`). It then re-derives the row via `upsert_library(lib_name, "npm", …)`
(`libraries.rs:158-159`), whose write is `ON CONFLICT(ecosystem, name)` (`pg_store.rs:3724`). For a
`(cargo, foo)` / `(pypi, foo)` / `(go, foo)` library this does **not** update the real row — it
**inserts a phantom `(npm, foo)` row**, attaches the refreshed pages there, and leaves the real row
(which `list_library_project_pins` reads, `pg_store.rs:3860`) stale. The applied-marker lands on the
phantom, so `should_reindex` stays true and the scheduler **re-enqueues forever**; worse, the
clobbered ecosystem makes the next `src.latest('npm', foo, …)` query the wrong registry and can
fabricate a spurious "update available". The reviewed design's D5 "COALESCE-preserve ecosystem" is
**impossible** — `ecosystem` is part of the conflict key; no COALESCE can recover it.

**Proposal.** On the re-index path, parse `task.folder_path` as a UUID; if it resolves to an existing
library, use that row's id + real ecosystem + source, and update-by-id — **never re-upsert by
`(name,'npm')`**. Extend `get_library` (or add `get_library_source(lib_id) → (source_type, base_url,
local_path)`) so the handler has the real ecosystem/source. Preserve the `add_library` first-index
behavior for genuinely-new libraries (D1).

**Owner files.** `crates/senseid/src/tasks/handlers/libraries.rs:140`,
`crates/senseid/src/db/pg_store.rs:3915`. **Effort:** M. **Risk:** med (shared handler).

### 0b. Success ≠ `Ok` — stamp the marker on `pages_stored > 0` only (correctness — HIGH)

**Problem.** `index_library` returns `Ok(pages_stored)`; per-page store failures are swallowed to
`tracing::warn` (`libraries.rs:187-188`) and `resolve_library_pages` may legitimately return zero
pages. So `Ok(0)` means *nothing was refreshed*. Stamping `docs_applied_version` on `Ok(_)` would
fabricate "applied", permanently suppress retry, and let the tick claim docs were refreshed.

**Proposal.** Stamp `docs_applied_version` / `docs_applied_at` (props) **only when
`pages_stored > 0`** (and `resolve_library_pages` returned a non-empty set). Treat `Ok(0)` as a
skip/failure so the next daily tick self-heals.

**Owner files.** `crates/senseid/src/tasks/handlers/libraries.rs:170-217`. **Effort:** S. **Risk:** low.

### 0c. Scheduler plumbing + mode-aware dedup (DRY + correctness — HIGH)

**Problem A (DRY).** The reviewed design proposed a new `ReindexEnqueuer` trait "for testability."
That reinvents shared infra: every sibling scheduler threads a concrete `Arc<TaskQueue>` —
`reconcile_scheduler::spawn(queue, pg)` (`reconcile_scheduler.rs:111`),
`analyzer_scheduler::spawn(queue, pg)` (`analyzer_scheduler.rs:160`), wired at `server.rs:231/264` —
and `TaskQueue` is an in-memory struct unit-tested via `has_pending_kind_path` (`queue.rs:339,570`).
Only `library_update_scheduler::spawn(Arc::new(state.pg.clone()))` (`server.rs:281`) omits it.

**Problem B (dedup).** `pending_library_update_exists` matches **any** status (incl. dismissed, per
its doc-comment) and is blind to `based_on.mode`/`is_security` (`pg_store.rs:3900-3912`). A premature
or dismissed low-urgency notify for a `to_version` would suppress a later `apply_failed` audit or a v2
high-severity security flag for that same version.

**Proposal.**
- Drop the trait. Change `spawn`/`run`/`tick` to take `Arc<TaskQueue>` (`server.rs:281 → spawn(pg,
  task_queue.clone())`); test the apply arm with a real `TaskQueue::new()` + `has_pending_kind_path`.
- Extend `list_library_project_pins` to also `SELECT l.base_url, l.source_type` (columns exist) so
  the tick can build `task.url`; fail-closed skip-refresh (but still flag) if no source resolves.
- Add `set/get_library_docs_applied` props helpers (single-statement `props || jsonb_build_object`,
  copied from `set_library_latest_cache:3874`).
- Give `pending_library_update_exists` an optional `mode`/`is_security` discriminator in the `@>`
  containment so security is never suppressed by a prior/dismissed non-security row.
- Guard overlap on the **lib_id** (`task.folder_path`), not the name (`has_pending_kind_path` keys on
  `task.path` = name, which isn't unique across ecosystems).

**Owner files.** `crates/senseid/src/api/server.rs:281`,
`crates/senseid/src/tasks/library_update_scheduler.rs:30-46`, `crates/senseid/src/db/pg_store.rs:3856,3903`.
**Effort:** M. **Risk:** low.

---

## 1. F-v1a — auto-apply PATCH (+ local capability refresh)

**Problem.** The pure core already returns `AutoApply` for `Bump::Patch` (`version.rs:68`, tested
L108), but the tick collapses it to a notify: `if update_action(bump,false) == Ignore { continue }`
(`library_update_scheduler.rs:88`) and both Notify and AutoApply fall through to
`create_recommendation_full`. Nothing applies.

**Proposal.** Replace the `== Ignore` short-circuit with a 3-arm match on `update_action(bump,
is_security)`:
- `Ignore` → `continue` (UNCHANGED — keeps `Unknown`/range/PEP440/sha pins fail-closed).
- `Notify` (minor/major, non-security) → today's `create_recommendation_full` path.
- `AutoApply` (patch) → the apply arm below.

**Apply arm (per DISTINCT library, because docs/caps are global):**
1. Resolve the re-index source from the extended pins (`base_url` else `local_path`). **No resolvable
   source → skip the refresh, still write a notify recommendation** (`mode='notify_no_source'`) so the
   patch stays visible; never fabricate a URL.
2. Pure gate `should_reindex(props.docs_applied_version, latest)`: true iff marker absent or
   `!= latest`. Skip if already applied. Guard in-flight duplicates with
   `has_pending_kind_path(IndexLibrary, lib_id)`.
3. Enqueue `Task::new(IndexLibrary, lib_id, name).with_url(url)` (`mcp.rs:184-190` recipe).
4. **Do NOT write an "applied" audit at enqueue time.** The re-ingest is async and may fail. The
   applied notice (`based_on.mode='auto_applied'`, per project pin, deduped) is written **only after a
   confirmed successful re-index** — i.e. driven off the props marker flipping (`should_reindex`
   becomes false because `index_library` stamped it on `pages_stored>0`), observed on a subsequent
   tick. Repeated failure surfaces `mode='apply_failed'` (now visible because dedup is mode-aware).

F3 capability refresh falls out of `index_library`'s `replace_library_capabilities`
(`libraries.rs:200-207`) — **for LOCAL libraries only**; remote libs refresh docs, not skills/agents
(D6). "Regenerate skills" means re-ingest manifest-declared capabilities; true skill *generation* is
unbuilt and out of scope.

**Owner files.** `crates/senseid/src/tasks/library_update_scheduler.rs:46-116`,
`crates/senseid/src/tasks/handlers/libraries.rs:140` (via 0a/0b). **Effort:** M. **Risk:** med.

**Tests.** `should_reindex` truth table; patch bump + LOCAL source → exactly one enqueue, no audit at
enqueue, audit appears only after the marker flips; second tick idempotent; no-source patch → notify
only; minor/major → notify only; range pin → Ignore arm; cargo re-index preserves ecosystem + creates
no phantom row; marker stamped on `Ok(>0)` only, not on the url-empty Err.

---

## 2. F-v1b — compat-gated MINOR (conservative notify fallback)

**Problem.** A real compat gate needs version-aware library API signature diffing. **It is unbuilt:**
`drift_items.expected_signature/actual_signature` (design.dbml:2429-2430) are never written;
`analysis/doc_drift.rs` is doc-vs-code symbol-name history, not cross-version library diffing; no
per-version library API surface is stored anywhere. Auto-applying a minor on a fabricated "clean"
would violate invariant #1.

**Proposal (ship the conservative path).** MINOR stays a **notify/review item** — the v0 behavior,
which already writes the `library_update` recommendation. Do **not** build `compat.rs`, a
`CompatProbe`, or a `minor_autoapply` flag in v1: with no real probe they are inert scaffolding whose
only reachable outcome is byte-identical to the existing v0 Notify arm (skeptic DRY finding). Optional,
best-effort enrichment: attach a blast-radius list from `get_callers_by_name` (`pg_store.rs:1774`) to
the review item as **context only, never a gate** (it is name-based, not version-aware).

When the future signature-diff pipeline (§ sequencing step 6) lands, add a **pure**
`compat_verdict(Option<&CompatReport>) → {Clean, NeedsReview, Unknown}` mirroring `version.rs`, where
`Clean` requires *positive completeness* (a completed full-surface analysis), never mere absence of
findings, and the "version_range still satisfiable + skills still resolve" heuristic may only downgrade
to `NeedsReview`/`Unknown` — never yield `Clean`.

**Owner files.** `crates/senseid/src/tasks/library_update_scheduler.rs` (Minor arm = existing v0 path).
**Effort:** S. **Risk:** low.

---

## 3. F-v2 — security scan + expedited apply (NEVER touches code)

**Problem.** No advisory/vuln source exists in `crates/senseid` (grep confirms only governance
"advisory" strings). The `is_security` seam in `update_action` (`version.rs:64,67`) is present but the
sole caller passes `false` (`library_update_scheduler.rs:88`).

**Proposal.** Add a net-new pure module `crates/senseid/src/libraries/advisory.rs` mirroring
`registry.rs`:
- `trait VulnSource { async fn advisories(ecosystem, name) -> Option<Vec<Advisory>> }` — **version-less
  query** (returns the library's full advisory set with affected ranges). `None` = undetermined
  (fail-closed); empty = determined-clean. `struct OsvVulnSource` (OSV.dev `/v1/query`, no auth,
  covers npm/PyPI/crates.io/Go).
- Pure, unit-tested helpers: `osv_ecosystem(sensei_eco) → Option<&str>` (npm→npm, pypi→PyPI,
  cargo→crates.io, go→Go; unmapped→None); `osv_query_body`; `extract_advisories`;
  `is_high_severity(&Advisory)` (parse CVSS_V3/V4 vector **and** `database_specific.severity`;
  indeterminate → not-high, log); `advisory_fixed_by(&Advisory, available)`;
  `security_verdict(&[Advisory], current, available) → {is_security, top}` where `is_security` = some
  HIGH-severity advisory that **affects `current`** (from full affected_ranges) **and is fixed
  at/below `available`**. Reuse `version.rs::parse_semver` + the `semver` crate — do not hand-roll a
  parser.

**Tick flow (per pin, extending §1).**
1. `bump = classify_bump(version_used, latest)`; if `None`/`Unknown`, skip (no OSV call — double-guards
   never-invent-a-bump).
2. For a REAL bump: read the **version-less** advisory set from `libraries.props` (TTL-cached, separate
   shorter TTL than the 23h version cache so a fresh CVE isn't throttled), else query `VulnSource` once
   per distinct library and cache the raw set. **Recompute `security_verdict` PER PIN** from the
   pin's `current_version` — never cache the boolean verdict per library (a single `libraries.props`
   row would cross-contaminate two projects pinning the same lib at different versions — HIGH).
3. `action = update_action(bump, is_security)` — UNCHANGED.
4. `AutoApply` → the **same** docs/skills refresh arm as §1 (routes to `index_library`, never a code
   change) **plus** a high-urgency (`urgency='high'`, the enum ceiling) `library_update` recommendation
   carrying `based_on.security` + `based_on.library_update.is_security=true`. The security **flag is
   written even if no re-index source resolves** — the flag is the user-critical output.

**Security dedup.** A security escalation must NOT be suppressed by a prior/dismissed non-security
notify for the same `to_version`. Use the mode/is_security-aware `pending_library_update_exists` (0c):
create a distinct high-urgency row when none with `is_security=true` exists (and re-open, don't reuse,
a dismissed low-urgency row).

**Hard guardrail.** The AutoApply arm's only side effects are (a) enqueue `IndexLibrary` (docs pages +
LOCAL manifest capabilities) and (b) write a recommendation + props marker. It **never** writes
`referenced_libraries.version_used` or any manifest/lockfile. A guardrail test asserts `version_used`
is unchanged after a security tick, and that no phantom `(npm,name)` row is created for a non-npm lib.

**Fail-closed points.** OSV unreachable/parse-miss → `None` → `is_security=false` → safe v1 behavior
(notify); unmapped ecosystem → None; indeterminate severity → not-high (degrade to notify-with-review,
log); `None`/`Unknown` bump → Ignore even with `is_security=true`.

**Owner files.** `crates/senseid/src/libraries/advisory.rs` (new), `crates/senseid/src/libraries/mod.rs`
(`pub mod advisory;`), `crates/senseid/src/tasks/library_update_scheduler.rs`,
`crates/senseid/src/db/pg_store.rs` (advisory props helpers + dedup extension),
`crates/senseid/src/api/server.rs` (wire `OsvVulnSource`). **Effort:** L. **Risk:** med.

---

## DDL changes

**Zero.** Apply-marker → `sensei.libraries.props` (jsonb, `libraries.ddl:18`); advisory raw set →
same props; compat verdict / apply mode / advisory payload → `inference.recommendations.based_on`
(jsonb); flags/TTLs → `sensei.config`. `action_type` stays free-text `'library_update'`; urgency uses
the existing `'high'`. Deliberately **not** adding `recommendation_status='auto_applied'` (would
pollute FTR semantics and trip the dbd alphabetical-enum-order gotcha) — auto-apply is recorded via
`based_on.mode` + a props marker, status stays `'pending'`.

---

## Sequencing

| Step | Item | Effort | Risk |
|---|---|---|---|
| 1 | **Prereq:** `index_library` resolve-by-lib_id + real ecosystem; marker on `pages_stored>0` | M | med |
| 2 | **Prereq:** thread `Arc<TaskQueue>`; extend pins (base_url/source_type); props marker helpers; mode-aware dedup | M | low |
| 3 | F-v1a auto-apply PATCH (3-arm match; audit only after confirmed success) | M | med |
| 4 | F-v1b compat-gated MINOR = conservative notify (no compat.rs/flag yet) | S | low |
| 5 | F-v2 security scan (`advisory.rs` + OSV; per-pin verdict; docs/skills + flag only) | L | med |
| 6 | **Future/out-of-scope:** per-version API-surface snapshot + signature diff → real `CompatProbe` | L | high |

---

## Open decisions

- **D1** — Fix only the re-index-by-lib_id path now, or also the `add_library` first-index `'npm'`
  hardcode / backfill? *Rec: fix re-index now; separate ticket for first-index; no backfill needed.*
- **D2** — Security auto-apply: docs/skills + flag only (rec), or a future opt-in dep-bump-PR feature?
- **D3** — `auto_applied` Observatory chip (one additive enum value) or `based_on.mode` (rec, zero DDL)?
- **D4** — Advisory cache: version-less raw set + per-pin recompute (rec) vs `(library_id,
  current_version)` key?
- **D5** — Severity threshold (CVSS ≥ 7.0 / HIGH) and handling of advisories with no CVSS vector.
- **D6** — Remote-library capability refresh scope (LocalDir-only in v1/v2; remote manifest fetch
  deferred).
- **D7** — Minor-autoapply flag scope (deferred with the whole apply-if-clean branch).

---

## Risks

- `index_library` is a shared primitive; the by-lib_id fix must preserve the `add_library` first-index
  path (`mcp.rs:184`) for genuinely-new libraries — regression risk if the UUID-resolution branch
  swallows the create case.
- The in-memory `TaskQueue` does not survive daemon restart: an enqueued auto-apply lost to a restart
  is safely re-created on the next tick ONLY because the applied-marker is stamped on confirmed success
  (`pages_stored>0`) — if that gate regresses, restart ⇒ duplicate work or a stale "applied" claim.
- Auto-apply patch churn: constraint #2 means `version_used` never changes, so `classify_bump` keeps
  returning Patch every tick; correctness of the `props.docs_applied_version` gate is load-bearing to
  avoid a daily re-enqueue forever.
- v2 OSV false-negatives are silent by design (fail-closed suppresses): a mis-mapped ecosystem label or
  unparseable severity yields no alert with no error surfaced — mitigate with unit tests over sample
  OSV bodies and a debug log on every skip.
- The "security escalates minor/major to AutoApply" policy (`version.rs` L112-113) auto-refreshes
  docs/skills on a security minor — acceptable ONLY because apply never touches project code; the
  invariant must be asserted in the apply dispatcher and a guardrail test, or it becomes dangerous if
  the apply target ever widens.
- `based_on` jsonb is now load-bearing for compat/security/apply-audit with no column constraints; a
  key typo silently breaks dedup — serialize via one typed Rust struct, not scattered `json!` literals.
- OSV `/v1/querybatch` returns only ids+modified stamps, not severity/affected/fixed data; the pure
  helpers need full `/v1/query` bodies, so batching cannot be "one call per tick" for detail —
  standardize on per-distinct-library `/v1/query` with the props TTL cache.

## Related
- [[spec/pipeline/library-intelligence]] — ingestion / skill-gen / version model (F builds on it)
- [[spec/2026-07-31-sensei-evolution]] — Part F (this is its deferred v1/v2)
- `crates/senseid/src/libraries/{version,registry,manifest}.rs` — the F v0 core reused unchanged
