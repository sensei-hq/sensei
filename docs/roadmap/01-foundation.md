# Roadmap — Foundation plane (M1–M6)

The floor: provisioning (M1), preferences (M2), capture (M3), the code graph
(M4), inference routing (M5), the data model (M6). Status legend:
[`README.md`](README.md).

> **Status populated 2026-11-03.** The cells follow the readiness table in
> [`../review/2026-11-03-orientation.md`](../review/2026-11-03-orientation.md) §2 and the capability
> audit in [`../review/2026-10-27-roadmap-vs-state.md`](../review/2026-10-27-roadmap-vs-state.md) §1.1.
> The last column names what proves each cell; a status is a snapshot, so move it when the module moves.

| ID | Module | One-line "done" | Status | Evidenced by |
|---|---|---|---|---|
| M1 | Bootstrap & Provisioning | Fresh install reaches all-green with no manual toolchain work | 🟡 **Partial** | `GET /health` resolves its six gates ready and hardware tier → model provisioning is built (`crates/bootstrap/src/health/resolvers/*.rs`, `/api/gateway/models/provision/status`); but releases are ad-hoc signed, so Keychain and folder permissions are asked again on every upgrade (#282) |
| M2 | Setup & Preferences | Every default is adjustable in a searchable surface — incl. icon style (kanji ⇄ solar) and the wider preference surface | 🟡 **Partial** | 11 panes under `app/src/routes/(preferences)/settings/`, now a window of their own; kanji ⇄ icons is live (`ui.glyphs`, #133); no search across settings (objective F3 unmet) and `/api/preferences` is 404 — preferences live in `/api/config` |
| M3 | Capture & Sessions | Every assistant session is captured with turns, tools and outcome; Claude + OpenCode first, extension-ready | 🟡 **Partial** | Claude hooks plus six adapters (`crates/senseid/src/transcript/{claude,opencode,copilot_cli,vscode,cursor,zed}.rs`), and the extension seam is `trait TranscriptAdapter` (`transcript/mod.rs`); Copilot's tool success and turn time are not captured and the mods migration (C-13) is unverified |
| M4 | Code Graph & Indexing | A repo scans to a resolved, queryable graph, incrementally | 🟡 **Partial** | Per-language adapters, incremental re-index (`watcher/root_watcher.rs`) and the nightly placement floor (`.github/workflows/placement-floor.yml`) are in; call resolution is 18% with no target (#146), and config/generated file classes are not classified |
| M5 | Inference Gateway | Every LLM step routes to a local or paid model under a policy | 🟡 **Partial** | `/api/gateway/{chains,status,consensus}`, 10 adapters listed by `/api/gateway/status`; budget cuts and the breaker surface are absent (`handlers/gateway.rs:112` `budget: None`) and there is no `inference_calls` cost ledger |
| M6 | Storage & Data Model | The schema is the single source of truth and drifts clean | ✅ **Built** | 487 `.ddl` files; `crates/bootstrap/tests/deploy_upgrade.rs` covers deploy and upgrade; rust.yml plans every SQL statement against the deployed schema; `tasks/activity_pruner.rs` prunes activity |

## Known gaps affecting this plane

| Gap | What | Priority |
|---|---|---|
| ~~A-1~~ | ~~app is kanji-only~~ — the kanji ⇄ icons switch is live (`ui.glyphs`, Settings → General; #133) | — |
| A-2 | the personal-preference surface is narrower than the configuration the app already supports (M2) | P3 |
| C-11 | transcript-scanning review not scoped as a task (M3) | P1 |
| C-13 | Claude "mods" (supersedes plugin + hooks) — install path + migration not verified (M3) | P1 |
| ~~C-14~~ | ~~multi-assistant expansion: seam unspecified~~ — **closed:** `pub trait TranscriptAdapter` (`transcript/mod.rs`) is the seam, and the six adapters are pinned by `assert_eq!(ads.len(), 6)`; the next adapters are open work, not an unknown seam (#348, #349) | — |

Full register: [`06-gaps.md`](06-gaps.md).

## Sequencing

M6 (data model) is the read base; M4's indexer state gates everything downstream
(graph 40.3% resolved, 1.6M/2.4M edges on the live sensei DB — see
`../history/reports/` for the 2026-10-13 placement numbers). C-13/C-14 (M3) are
the facing capture work this quarter.
