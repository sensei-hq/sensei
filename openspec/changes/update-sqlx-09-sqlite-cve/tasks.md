## 1. sensei
- [ ] 1.1 `dbd-core` without its `sqlite` feature; `cargo tree -i libsqlite3-sys` shows rusqlite alone
- [ ] 1.2 rusqlite 0.43 (libsqlite3-sys 0.40); transcript reader tests unchanged and green
- [ ] 1.3 Remove the audit ignore + the backlog exception; `cargo audit` clean (shown failing first with the ignore gone)
## 2. Verification
- [ ] 2.1 Full senseid suite (DB) and clippy green; install the built daemon and re-run a transcript sync
- [ ] 2.2 `openspec validate update-sqlx-09-sqlite-cve --strict`
