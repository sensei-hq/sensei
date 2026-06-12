# sensei-hive

The org "shared brain" for sensei governance federation — a slim Axum service over an embedded Postgres holding promoted, shareable rules.

## Run

```bash
# First, mint an admin key (creates the embedded DB on first run):
sensei-hive keygen --name "admin" --role admin --label initial
# Then serve:
SENSEI_HIVE_BIND=0.0.0.0:7755 sensei-hive serve
```

Config (env): `SENSEI_HIVE_DATA_DIR` (embedded PG data dir, default `~/.sensei-hive/pg`), `SENSEI_HIVE_DDL_DIR` (the `database/` DDL tree), `SENSEI_HIVE_BIND` (default `127.0.0.1:7755`).

Schema = the `hive` dbd scope (`database/design.yaml`): `shared_rules`, `members`, `api_keys`, `audit_log` + the shared `scopes`/`namespaces`/`enforcement`. Roles: `member` (pull), `publisher` (+publish), `admin` (+manage). See the [federation design spec](../../docs/superpowers/specs/2026-06-11-hive-mind-federation-design.md).
