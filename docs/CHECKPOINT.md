# Checkpoint — 2026-10-25

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Wave 4 is closed; wave 5 is in progress.
Released v0.20.0 (dōjō deployed; Worker `/version` = 0.20.0 `fcf5d87`) and v0.20.1 (daemon only).

**Closed this run:** #106, #335, #201, #204, #183, #142, #298, #172, #195 (#120 updated).
- Dependencies: Rust 16 → 0 actionable advisories; Tauri lockfile 10 → 0; JS critical/high cleared.
  `cargo audit` gates rust.yml over every lockfile; exceptions in `.cargo/audit.toml`.
- dbd 0.27.6 and kavach 1.3.0 released; the dōjō runs kavach 1.3.0, verified live.

**In progress — #178 (wave 5, B1), landed on develop (2e5c6c1f):**
- A file over 2 MiB, or with a line over 10,000 bytes, is skipped as `oversized`. This is a new
  `scan_skip_reason` value.
- `process_file` now uses the scan's `classify_unscannable`.
- Local live DB reconciled (enum has `oversized`). Over the 61,349-file corpus the classifier refuses
  exactly 137 files.
- Left: after the next release installs, `select count(*) from sensei.files where
  skip_reason='oversized'` > 0, then close #178.

**Open (the user's):**
- dbd 0.27.7: the v0.27.6 lockfile still pins rustls-webpki/webpki-roots/ctutils for
  `cargo install --locked` (fixed on dbd develop, 784db3d2). Needs a go.
- v0.20.1's `deploy-dojo` is still waiting; it would change nothing. Approve or reject.
- #331: count production's duplicate personal dōjōs (counts only):
  `psql "$PROD" -Atc "select count(*) from dojo.tenants where origin='personal' and key ~ '-[0-9]+$'"`.

**Known:** DeepSource is red on kavach main; qlty is out of minutes (dbd, sensei). After any DDL
change, run `make test-template` (the suite refuses a stale template, #142).

**Next:** #188 (B2), surfacing watchdog abandonment. Start by reading the executor watchdog and
counting abandonments in `public.logs`.
