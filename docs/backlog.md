---
name: Implementation Backlog
description: Prioritized index of work — open items are tracked as GitHub issues (sensei-hq/sensei)
date: 2026-04-28
---

# Implementation Backlog

Work is tracked as **GitHub issues** in [`sensei-hq/sensei`](https://github.com/sensei-hq/sensei/issues), tagged `bug` / `feature`. This file is the prioritized index. When an item ships, **close its issue and remove it from this list**. Detailed engineering history from before this migration (2026-06-04) lives in git (`git log -- docs/backlog.md`).

---

## 1. Top priority — Governance plane

**Concept:** [concepts/governance.md](./concepts/governance.md) · **Epic:** [#28](https://github.com/sensei-hq/sensei/issues/28) · **Builds on:** [knowledge plane spec](./superpowers/specs/2026-05-27-knowledge-plane-design.md)

Layered scope×enforcement rule model, `scopes`+`namespaces` (level-based set membership), global `~/.sensei/rules.md`, README-frontmatter identity, two-tier resolution via the `consolidation` inference role, promotion, and the slim hive-mind federation service. Ships top-to-bottom:

- **P1 — Tool routing** (no schema change, first): [#11](https://github.com/sensei-hq/sensei/issues/11) agents→MCP tools · [#12](https://github.com/sensei-hq/sensei/issues/12) `get_duplicates`/`get_project_conventions` · [#13](https://github.com/sensei-hq/sensei/issues/13) `~/.sensei/rules.md` + CLAUDE.md pointer · [#14](https://github.com/sensei-hq/sensei/issues/14) agents.md doc
- **P2 — Scopes/namespaces/resolution**: [#15](https://github.com/sensei-hq/sensei/issues/15) [#16](https://github.com/sensei-hq/sensei/issues/16) [#17](https://github.com/sensei-hq/sensei/issues/17) [#18](https://github.com/sensei-hq/sensei/issues/18) [#19](https://github.com/sensei-hq/sensei/issues/19) [#20](https://github.com/sensei-hq/sensei/issues/20)
- **P3 — README identity + promotion**: [#21](https://github.com/sensei-hq/sensei/issues/21) [#22](https://github.com/sensei-hq/sensei/issues/22) [#23](https://github.com/sensei-hq/sensei/issues/23) [#24](https://github.com/sensei-hq/sensei/issues/24)
- **P4 — Hive-mind federation**: [#25](https://github.com/sensei-hq/sensei/issues/25) ✅ `sensei-hive` service · [#26](https://github.com/sensei-hq/sensei/issues/26) ✅ daemon federation module (`knowledge_sources` + `federated_memories` ledger, push-on-approve + poll-pull; federated rules are ordinary `memories(origin='federated')` so resolution is unchanged) · [#27](https://github.com/sensei-hq/sensei/issues/27) Configure UI (remaining)

---

## 2. Daemon — ingest & scan correctness (bugs)

| Issue | Summary |
|-------|---------|
| [#29](https://github.com/sensei-hq/sensei/issues/29) | Subfolders auto-promoted to projects (discovery resumer treats every walked dir as a project root) |
| [#30](https://github.com/sensei-hq/sensei/issues/30) | Rokkit not detected as a library (scoped `@rokkit/*` packages missed by resolver) |
| [#31](https://github.com/sensei-hq/sensei/issues/31) | `activity.sessions` empty despite Claude hooks firing |
| [#32](https://github.com/sensei-hq/sensei/issues/32) | Queue activity event hardcodes `elapsed=0.0` (`process.rs:80`) |
| [#33](https://github.com/sensei-hq/sensei/issues/33) | Scan live-feed: emit `Process`-level activity events during indexing + align discover/summary message vocabulary (`quasi-repo`→`standalone`). **Down-scoped** — the original two halves shipped: client counts by `level`/`status` (no message scraping, `scan-state.svelte.ts`) and `progress_emitter.rs` emits running `folder_update`/final `indexed`/`project_update→active`. Remaining gap: `ActivityLevel::Process` is defined but never emitted, so the textual activity log goes silent through the indexing phase even though the bars animate. |
| [#34](https://github.com/sensei-hq/sensei/issues/34) | Test fixture projects pollute the dev DB (`ensure_test_project`) |
| [#35](https://github.com/sensei-hq/sensei/issues/35) | Scan stats: queued cumulative vs processed current-state |
| [#62](https://github.com/sensei-hq/sensei/issues/62) | Single repo w/ multiple folders misclassified as multi-repo — every folder promoted to a repo + generic tags (Acolytes). Needs root-cause analysis |
| [#63](https://github.com/sensei-hq/sensei/issues/63) | rokkit monorepo: only some `@rokkit/*` packages detected as libraries (#30 follow-up — partial fix) |
| [#64](https://github.com/sensei-hq/sensei/issues/64) | Task-queue "blocked" logs don't identify what was blocked or why |
| ✅ SHIPPED + DEPLOYED (2026-06-17) | **`node_kind` enum silently dropped `doc`/`struct`/`component`/`hook`/`extension` nodes** (`upsert_node` enum cast failed; `.ok()` swallowed). Fixed on develop (`9b01da2f`→`95af9848`, DDL `171ad9b4`): 5 kinds added to enum, dead variants removed, guard test, embedding allowlist widened, `.ok()` swallow → `tracing::warn!`. Prod reset+rebuilt + validated live (doc 263, hook 152, component 103, struct 5, extension 49; covers 150, references 3421 — all were 0). Plus export/import infra for capture preservation. Full `/Users/Jerry/Developer` rescan still running. Spec/plan: `docs/superpowers/{specs,plans}/2026-06-16-node-kind-drops*`. |
| _(follow-up)_ | **Codebase-wide silent-error audit.** Find & fix every place that discards an error without logging (`.ok()`, `let _ =`, empty catch, masking `unwrap_or_default`) — errors must be logged so they can be inspected/rectified. Directed after the node_kind fix; the latter is one instance. |

## 3. Setup wizard

| Issue | Summary |
|-------|---------|
| [#36](https://github.com/sensei-hq/sensei/issues/36) | Stage regressions (2026-05-27 smoke) — **may be partly fixed by the wizard rehab; triage and close verified items** |
| [#37](https://github.com/sensei-hq/sensei/issues/37) | Build out placeholder + deferred stages (Projects/Libraries/Instruments; Inference/Assignments need gateway design) |

## 4. Observatory

| Issue | Summary |
|-------|---------|
| [#38](https://github.com/sensei-hq/sensei/issues/38) | Learnings vs Insights overlap — decide `/insights` purpose |
| [#61](https://github.com/sensei-hq/sensei/issues/61) | Recent sessions renders empty/ghost rows — session API contract drift + render contentless rows. **Fixing now** |

## 5. Daemon pipeline & API + tooling (features)

| Issue | Summary |
|-------|---------|
| [#40](https://github.com/sensei-hq/sensei/issues/40) | Scan pipeline: progress + edge/connection events (pairs with #33) |
| [#82](https://github.com/sensei-hq/sensei/issues/82) | `detected_patterns` + FTR/pattern-effectiveness should be **project-scoped**, not folder-scoped (folder-level derivation misses project-wide patterns; FTR should be measured per-project). Surfaced during corrections-aggregation design. |
| _(standalone #65 step 5)_ | **Corrections aggregation — SHIPPED + LIVE-VERIFIED on `develop`.** New `inference.corrections` table + global `AggregateCorrections` task (once per analyzer tick): clusters recurring corrective prompts across projects (embedding→lexical fallback, deterministic seed signature, idempotent upsert+prune), per-cluster LLM canonical text/suggestion/memory link (graceful), `GET /api/corrections` + `/api/projects/{id}/corrections`. Spec/plan in `docs/superpowers/{specs,plans}/2026-06-24-corrections-aggregation*`. Live (debug daemon): pipeline runs `analyze_project→derive_signals→aggregate_corrections` (logged `0 upserted, 0 pruned` — correct: corpus has only ~15 scattered corrections, none cluster ≥2-similar); endpoints return `{"corrections":[]}`. Logic proven by unit + DB-integration tests. Will populate as recurring corrections accrue. |
| [#65](https://github.com/sensei-hq/sensei/issues/65) | **Epic** — Periodic session/log analyzer → findings, learnings/memory, recommendations. Blueprint+plan done. **SHIPPED:** [#66](https://github.com/sensei-hq/sensei/issues/66) L0 enrichment · [#67](https://github.com/sensei-hq/sensei/issues/67) scheduler · [#68](https://github.com/sensei-hq/sensei/issues/68) L1 SignalDeriver (churn + correction + rule-candidate signals — tool failures aren't captured, so behavioral) · [#72](https://github.com/sensei-hq/sensei/issues/72) FTR endpoint NUMERIC fix · [#73](https://github.com/sensei-hq/sensei/issues/73) transcript backfill v1 (Claude) · [#69](https://github.com/sensei-hq/sensei/issues/69) **L2 Generator (`29329f32`)** — patterns/signals → recommendations + learned memories, heuristic + idempotent (based_on/source_id); wired into analyze_project, runs every pass. Shape-forks DDL first (`eda9f6a1`: memory category/created_at + recommendation based_on). LIVE: 171 recs (168 audit_stale + 2 write_skill + 1 promote_pattern, all with based_on) + 1 learned memory; idempotent re-run. [#71](https://github.com/sensei-hq/sensei/issues/71) **L3 maturity (`c49f1ebe`)** — `crate::maturity` derives early/mature (mature = ≥3 enriched sessions AND ≥1 insight) + watched/target; `GET /api/projects/{id}/maturity`; on-read, no DDL, doesn't touch the lifecycle enum. LIVE: ff1ccea2 mature (13 watched), low-activity early (2<3), fresh early (0). [#70](https://github.com/sensei-hq/sensei/issues/70) **L2 consolidation (`f84084d9`+`3fcfd240`)** — top findings → gateway `reasoning` chain (gemma4) → `reasoning_trace` + a recommendation linked via `reasoning_trace_id` with LLM `why`+`prompt`; idempotent via signature on `trigger_detail` (deterministic top-N w/ `dp.id` tiebreaker); degrades to no-op w/o a model. LIVE: consolidation traces (`models_used={gemma4}`) + linked `create_agent` recs w/ 400-char prompts; sequentially idempotent (frozen project stable). **🎉 #65 ANALYZER COMPLETE** (L0→L1→L2 heuristic→L2 consolidation→L3). **FOLLOW-UPS:** rework-rec volume tuning **DONE** (`c5bd2ea9`: recs gated to ≥8 re-edits; existing 168 dismissable) · consolidation TOCTOU race (concurrent same-project passes can dup a signature — add partial unique index `(project_id, trigger_detail->>'signature')` + ON CONFLICT for atomicity; rare under hourly scheduler) · #73 follow-ups (Zed adapter, forward Stop-hook, embeddings) |
| [#74](https://github.com/sensei-hq/sensei/issues/74) | Activity-data retention / pruning — periodic prune of `assistant_events` / `turns` / `sessions` / `transcript_turns` older than a configurable TTL (**default 30d**), only after analysis has derived insights (keep `detected_patterns`/recommendations/teachings/memories). GC counterpart to the #65 analyzer + #73 backfill. |
| [#75](https://github.com/sensei-hq/sensei/issues/75) | **v1 SHIPPED (v0.2.23)** — Cold-start the analyzer from historical transcripts: synthesize sessions + assistant_events from the transcript (a superset of the hook stream), reuse L0/L1 enrichment. Live: 19 historical sessions synthesized+enriched. Remaining: install-time auto-trigger, untracked-folder staging, subagents, Zed. |
| [#76](https://github.com/sensei-hq/sensei/issues/76) | **CORE SHIPPED on develop (`8ddcac94`)** — Table-driven gateway config from seed. Daemon now loads routers/models/**chains** from `gateway.*` (new `api/gateway_config_loader.rs`, pure builders + `load_gateway_config(pool)`; `init_gateway` loads from DB, baseline demoted to last-resort fallback + per-capability graft). Seed (`import/staging/*.jsonl`) rewritten embedded→ollama→cloud (router-gated): `llama-cpp-chat`/`llama-cpp-embed`/`nvidia` routers, `gemma2:2b` embedded chat + `nvidia llama-3.1` models; classify/summarize/reasoning lead with embedded, embed stays 384-dim (embedded→ollama all-minilm); consensus-* panel left intact. Callers pin named chains (classify→`classify`, governance merge→`reasoning`, embed→`embed`). `EMBED` now default-on for all `make` daemon builds (`EMBED=0` opts out) + `crates-all` gate; embed adapter resolves managed `~/.sensei/models/embed.gguf`. Live verified: seed imported to prod, loader reads embedded-first config (integration test green). **DEFERRED:** [#77](https://github.com/sensei-hq/sensei/issues/77) image-gen as seed (needs `model_capability` `image`; baseline-graft stopgap) · [#78](https://github.com/sensei-hq/sensei/issues/78) embedded in CI release binaries (cross-platform native llama.cpp — needs sign-off) · provision a 384-dim embed GGUF (or `ollama pull all-minilm`). |
| [#79](https://github.com/sensei-hq/sensei/issues/79) | **v1 SHIPPED + LIVE (develop `3a2a992a`)** — Unify embedded llama.cpp into a single `embedded-llama` router (was `llama-cpp-chat`/`llama-cpp-embed`). New multiplexing `EmbeddedLlamaAdapter` (gateway-embedded) holds `{(model_id,mode)→LlamaCppAdapter}` workers, dispatches by `request.model`, mode from payload, lazy `spawn_blocking` load via `ChainedResolver` (Managed → Ollama). `init_gateway` registers one adapter via the resolver (dropped hardcoded GGUF paths + `llama_cpp_init`); seed collapsed to one `embedded-llama` router with ollama resolver ids (gemma2:2b / all-minilm) so ollama-pulled blobs are reused in place — **no shipped/copied GGUF**. `/api/gateway/infer` gained an optional `chain` param. Live proof: `POST /infer {chain:classify}` → `model gemma2:2b` loaded from the ollama blob, generated in-process. **SHIPPED since:** embedded defaults (gemma2:2b + all-minilm) added to bootstrap `required_models` all tiers (`fb245a12`); chat **and** embed both verified live through the multiplexer (embed: all-minilm 384-dim). **Ollama-free provisioning SHIPPED + VERIFIED (`a896cc20`)** — `senseid::model_provision::Provisioner::ensure_model` pulls GGUFs from `registry.ollama.ai` over plain HTTP (no ollama daemon/CLI), sha256-verifies against the manifest digest, writes to the managed dir + registers in the index; integration test pulls all-minilm (~45MB) live. **REMAINING:** wire `ensure_model` into a startup/wizard trigger (UX decision: silent auto-pull vs gated); gemma4 (reasoning lead) left manual per user; reseed left harmless orphan `llama-cpp-*` router rows (cleared on next `dbd reset`). |
| [#80](https://github.com/sensei-hq/sensei/issues/80) | Gateway tier-3 capability resolution is non-deterministic (HashMap order) when multiple chains share a capability — found verifying #79. Generic `/infer` without a chain can pick any TextChat chain. Daemon tasks pin named chains so unaffected; fix by ordering on `fallback_chains.sequence` or a default-chain-per-capability. |
| [#41](https://github.com/sensei-hq/sensei/issues/41) | API alignment: missing endpoints (scan roots CRUD, libs configure, mcp registry/configure, projects merge) |
| _(file issue)_ | Surface stale / orphaned / unused projects & folders for cleanup. Scan reconcile already tags dead-but-ambiguous folders `stale` and empty projects `orphaned` (never auto-deletes). Needs: list endpoints + a gated purge action + a housekeeping UI (Observatory Configure or Projects setup). Last `~/Developer` rescan: 6 stale folders, 44 orphaned projects. |
| [#39](https://github.com/sensei-hq/sensei/issues/39) | Bootstrap diagnostic logging + debug mode (trace, log viewer, app menu, GitHub submit) |
| [#42](https://github.com/sensei-hq/sensei/issues/42) | E2E: configure Tauri-mode local E2E |

## 6. Design / mockup gaps (features)

| Issue | Summary |
|-------|---------|
| [#43](https://github.com/sensei-hq/sensei/issues/43) | Observatory Configure section (design + build) |
| [#44](https://github.com/sensei-hq/sensei/issues/44) | J7 Extend & Customize screens (design + build) |
| [#45](https://github.com/sensei-hq/sensei/issues/45) | J5 Pattern knowledge catalog (low) |
| [#46](https://github.com/sensei-hq/sensei/issues/46) | J9 Context pack tool (low) |

## 7. UI migration (quality)

| Issue | Summary |
|-------|---------|
| [#47](https://github.com/sensei-hq/sensei/issues/47) | Mockup component migration to Rokkit (steps 4–14) |
| [#48](https://github.com/sensei-hq/sensei/issues/48) | CSS migration to semantic tokens (largely superseded by #47) |

## 8. Future scope (deferred)

| Issue | Summary |
|-------|---------|
| [#49](https://github.com/sensei-hq/sensei/issues/49) | `gateway-embedded` crate — in-process inference adapters |
| [#50](https://github.com/sensei-hq/sensei/issues/50) | Extract `bootstrap` into a reusable library |

## 9. Known issues / limitations

| Issue | Summary |
|-------|---------|
| [#51](https://github.com/sensei-hq/sensei/issues/51) | ACP config: JSONC comment loss on rewrite |
| [#52](https://github.com/sensei-hq/sensei/issues/52) | Database DDL upgrades: no rollback for partial/failed upgrades (offload to dbd) |

## 10. Website (`website/`)

| Item | Summary |
|------|---------|
| ✅ FIXED | **Screens gallery used kanji `先` instead of the logo SVG.** `MockSidebar.svelte:17` (shared by all `Mock*` gallery components) hardcoded `<span class="kanji">先</span>` as the brand mark. Replaced with the sensei logo icon (`i-brand:sensei text-sensei`) + center-aligned `.logo`, matching the real page header at `routes/sensei/+page.svelte:120`. Verified via dev-server screenshot. |
| ⚠️ 1 known error (upstream) | **`svelte-check` baseline cleanup.** `bun run check` had 20 pre-existing errors; **19 fixed** (`app.d.ts`: `__APP_VERSION__` global + `ButtonProps.rel` augmentation + `@types/rokkit__states` shim via `typeRoots`; added `@types/node`; `HTMLElement` casts in `dark-mode.spec.ts`). **1 remaining is upstream rokkit** ([jerrythomas/rokkit#139](https://github.com/jerrythomas/rokkit/issues/139)): `@rokkit/ui`'s `CommandPalette.svelte` ships as plain JS → implicit-any TS7016 with no clean consumer-side fix. **Accepted exception** (user-signed-off) until #139 ships; then remove the `@types/rokkit__states` shim + `typeRoots` too (both only needed for the same upstream gap). |
| _(file issue)_ | **On-page SEO gaps** (from SEO checklist audit): (a) canonical tag missing — add `<link rel="canonical" href="…">`; (b) OpenGraph tags missing — `og:title`, `og:description`, `og:image`; (c) Twitter Card tags missing — `twitter:card`, `twitter:title`, `twitter:description`, `twitter:image`; (d) not yet indexed by Google — submit sitemap to Google Search Console and request indexing for key pages. Likely all belong in the root `+layout.svelte`/`app.html` `<svelte:head>` so every route inherits them, plus a generated `sitemap.xml`. |
