# Checkpoint — 2026-10-25

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md), wave 4. Released v0.20.0 (dōjō deployed;
Worker `/version` = 0.20.0 `fcf5d87`) and v0.20.1 (daemon only).

**Closed this run:** #106, #335, #201, #204, #183, #142, #298.
- #183: a database per test, all 15 gates removed; suite ~300–390s.
- #142: the suite refuses a template behind the DDL tree; run `make test-template` after a schema change.
- #298: Instruments 6–8.4s → 0.58s, on dbd 0.27.6.

**Releases today (user go, 2026-10-25):**
- **dbd 0.27.6** (crates.io; PR #106 merged to main): a new generated column is added generated; a
  generated CASE compares with its live form (dbd#104). dbd#105 filed (promoting a column a view reads).
  After the tag, three RustSec advisories (rustls-webpki, webpki-roots, ctutils) were fixed on dbd
  develop (784db3d2); the v0.27.6 lockfile still pins them for `cargo install --locked` →
  **0.27.7 recommended** (needs a go).
- **kavach 1.3.0** (npm, all 14 packages; PR #56 merged): sync once per session change (#54) and the
  built-in landing guard (#55). Dependency advisories 93 → 1 (dev-only undici in wrangler); the CI
  gate now audits what ships (`scripts/audit-shipped.mjs`, user decision); lockfile written by the
  pinned bun (Cloudflare and CI read it). DeepSource is red on kavach main (findings on its dashboard).
- **dōjō on kavach 1.3.0** (c86ebc81): its own landing guard removed; verified live (one session POST,
  the previous account never shown).

**Open:**
- v0.20.1's `deploy-dojo` is still waiting; it would change nothing (v0.20.1 is daemon-only). Approve or reject.
- #331: count production's duplicate personal dōjōs (counts only). Needs a production connection:
  `psql "$PROD" -Atc "select count(*) from dojo.tenants where origin='personal' and key ~ '-[0-9]+$'"`.
- qlty is out of minutes (dbd, sensei).

**Next (wave 4):** the dependency cluster #172/#195/#120. sensei's `Cargo.lock` has open RustSec
advisories (quick-xml, h2, quinn-proto, ctutils, …), and no workflow runs `cargo audit`.

**Next command:** `cargo audit` on sensei; bump the affected crates; add `cargo audit` to rust.yml.
