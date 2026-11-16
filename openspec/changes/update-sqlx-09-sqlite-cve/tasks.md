## 1. dbd (its repo)
- [ ] 1.1 sqlx 0.9 in dbd-core; its tests green; release a tag
## 2. sensei
- [ ] 2.1 sqlx-core/sqlx-postgres 0.9 + the dbd-core tag; fix API changes; full senseid suite (DB) green
- [ ] 2.2 rusqlite on libsqlite3-sys ≥ 0.39; transcript reader fixtures unchanged
- [ ] 2.3 Remove the audit ignore + the backlog exception; `cargo audit` clean
## 3. Verification
- [ ] 3.1 Install the built daemon and re-run a transcript sync; `openspec validate update-sqlx-09-sqlite-cve --strict`
