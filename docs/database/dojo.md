# `dojo` — design notes

Why the tables in `database/ddl/table/dojo/` are shaped the way they are.

What a column MEANS lives in `comment on column`, where the database catalog can serve it. This file holds the other half: what was measured, what was tried and failed, and what must not be changed back. It is prose about decisions, which is why it is not in the DDL.


## `dojo.repositories`

[`database/ddl/table/dojo/repositories.ddl`](../../database/ddl/table/dojo/repositories.ddl)


### `visibility`


The FORGE's answer, captured at sign-in. NULLABLE and with NO DEFAULT, and both of those are load-bearing:

"not captured" must be its own state, because the two consumers of this column have OPPOSITE safe defaults. Entitlement wants to assume `private` (do not host unknown code free); authority wants to assume `public` (do not treat unknown code as org-mandated). No single default is safe, so an uncaptured repository has no authority, no election and no sync.

Verified before this was written: under the old `not null default 'private'`, `github.com/sensei-hq/dbd` — PUBLIC on GitHub — resolved to ORG-MANDATED, and would have been shared with no election by anyone.


## `dojo.repository_elections`

[`database/ddl/table/dojo/repository_elections.ddl`](../../database/ddl/table/dojo/repository_elections.ddl)


A user's election and an org's mandate are DIFFERENT ROWS, so a repository going public (authority organization → user) does not silently convert one into the other. NULLS NOT DISTINCT so the org's single NULL-principal row collides with itself rather than duplicating.


## `dojo.repository_metrics`

[`database/ddl/table/dojo/repository_metrics.ddl`](../../database/ddl/table/dojo/repository_metrics.ddl)


### `principal_id`


WHO a scope='user' row belongs to. A principal, never a git email: a commit trailer is an unverified assertion — anyone can set user.email to a colleague's address — so attributing shared numbers by it would be an attribution attack. The email may travel in props; it must not be the key.


NULLS NOT DISTINCT, matching sensei.repository_metrics. Without it the constraint fires for NOTHING the daemon pushes: every repo-scoped row carries principal_id = NULL, and day-grain rows carry commit_sha = NULL, so under the default NULLS DISTINCT two byte-identical rows are both accepted. Verified against Postgres 17 before adding this — two identical inserts gave 2 rows.

That made idempotence rest entirely on a non-atomic select-then-insert in TypeScript: two machines pushing the same (metric, repo, day) both miss the SELECT, both INSERT, and every later push for that repository then fails on PGRST116 forever. This clause is what makes the re-push genuinely idempotent, and it is what spec claim C5 was credited with and did not have.
