# sensei-dojo

The org "shared brain" for sensei governance + artifact federation — a slim Axum service over an embedded Postgres holding promoted, shareable rules and Dōjō artifacts.

## Run

```bash
# First, mint an admin key (creates the embedded DB on first run):
sensei-dojo keygen --name "admin" --role admin --label initial
# Then serve:
SENSEI_DOJO_BIND=0.0.0.0:7755 sensei-dojo serve
```

Config (env): `SENSEI_DOJO_DATA_DIR` (embedded PG data dir, default `~/.sensei-dojo/pg`), `SENSEI_DOJO_DDL_DIR` (the `database/` DDL tree), `SENSEI_DOJO_BIND` (default `127.0.0.1:7755`).

Schema = the `dojo` dbd scope (`database/design.yaml`): the governance-federation tables (`shared_rules`, `members`, `api_keys`, `audit_log`) AND the artifact-federation tables (`artifacts`, `tenants`, `memberships`, …) + the shared `scopes`/`namespaces`/`enforcement`. Roles: `member` (pull), `publisher` (+publish), `admin` (+manage). See the [federation design spec](../../docs/superpowers/specs/2026-06-11-hive-mind-federation-design.md).
