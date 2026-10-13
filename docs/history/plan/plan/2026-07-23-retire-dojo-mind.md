# Retire the `dojo-mind` Rust service → Worker `/v1` (migration plan, 2026-07-23)

> Path A confirmed: the Dōjō SvelteKit **Worker `/v1`** is the only dōjō backend.
> `crates/dojo-mind` (`sensei-dojo` binary) is retired; `senseid` federation targets
> the Worker over HTTP. Shared wire types stay in `crates/dojo-protocol`.

## Key finding (corrects the premise)

- **`dojo-mind` is a `[dev-dependency]` of senseid** (`crates/senseid/Cargo.toml:127`), used by **one test** (`federation/mod.rs::e2e_daemon_pulls_a_rule_published_on_the_dojo`, `mod.rs:248`, which boots an in-process dōjō). The **runtime daemon never links it.**
- **senseid already talks to a dōjō over HTTP**, config-driven, on two planes:
  - **Plane A — rules federation:** `POST /v1/rules`, `GET /v1/rules?since=` to `KnowledgeSource.url`, auth = Keychain **API key** (`federation/mod.rs:82,102`).
  - **Plane B — artifacts + relay:** `POST/GET /v1/t/{tenant}/artifacts` + `relay/*` to `DojoMembership.registry_url`, auth = **device token** (`dojo/client.rs`).
- **relay is already in the Worker** (9 routes). **Gaps** the daemon needs: `/v1/rules` (Plane A) + `/v1/t/{tk}/artifacts` (Plane B) — neither exists server-side in the Worker yet. Console endpoints (admin/maintainer/lead) are **not called by the daemon** — human-console only.

So the daemon "already calls the Worker shape"; the work is (1) build the missing Worker endpoints, (2) replace the one embedded-dōjō test with an HTTP stub, (3) delete the crate.

## Sequence

1. **Worker daemon-facing endpoints** *(gated on D1)*
   - `POST /v1/rules` · `GET /v1/rules?since=` · `DELETE /v1/rules/{id}` (rules federation).
   - `POST/GET /v1/t/{tk}/artifacts` (contribute + downstream pull; incl. the promote sweep).
2. **Worker console endpoints** *(gated on D2)* — triage · members/identities/policies · incidents · engagements detail+bind · audit · compliance · health. JWT plane; `engagements/+server.ts` is the template.
3. **senseid test → HTTP stub** — rewrite the federation e2e test to stub `GET /v1/rules` with a tiny axum handler returning a `dojo_protocol::PullResponse` (mirror `dojo/client.rs:482`), then **drop the `dojo-mind` dev-dep**. Decision-free + safe (kills the embedded-PG flakiness too).
3b. Confirm the daemon's runtime URLs point at the Worker (`KnowledgeSource.url` / `DojoMembership.registry_url`).
4. **Delete the crate + refs** *(after 1–3)* — `crates/dojo-mind/`, `Cargo.toml:8` member, `Makefile` `dojo` target (`:79-81`) + header (`:10`), `coverage.yml` comment, `CLAUDE.md:13`, doc references (`docs/architecture/dojo*.md`, `governance.md`, `data.md`, `backlog.md`). **Keep `dojo-protocol`.**

**Verify per step:** Worker `bun run check`/`test` + route specs + wrangler smoke (1,2); `cargo test -p senseid federation` + `cargo build --release -p senseid` (3); `cargo build/test --workspace` + `make crates-all` + grep-clean for `dojo_mind|dojo-mind|sensei-dojo` (4).

## Decisions needed (before building the Worker endpoints)

- **D1 — `/v1/rules` auth model (the one real fork).** dojo-mind's `/v1/rules` is a **global, non-tenant API-key** plane (`dojo.members`/`dojo.api_keys`); the Worker only has the **per-tenant device-token** plane (`memberships.device_token_hash`). Options:
  - **(a) Recommended — move rules under the tenant path** `/v1/t/{origin}/{org}/rules`, reusing the Worker's existing device-token auth (`resolveApiKeyAccess`). The daemon's `KnowledgeSource` points at the tenant URL with a device token — unifies the daemon's dōjō auth on one plane, no new table.
  - (b) The Worker grows a global API-key table + middleware (keeps `/v1/rules` global). More surface, more to secure.
- **D2 — console endpoint scope.** Build all console `/v1` routes now, or only those with a live UI consumer? (The daemon needs none of them.)
- **D3 — artifacts server scope.** The daemon already calls `/v1/t/{tk}/artifacts` with no live server. Build it now (needed for live contribute/downstream) or defer?

## Safe-to-start-now (decision-free)
Step 3 (senseid test → HTTP stub + drop the dev-dep) doesn't delete `dojo-mind` and needs no decision — it just removes senseid's last coupling to the crate and de-risks the eventual deletion.
