# `tests/` — assertions that run against a real Postgres

## Why this exists

Every other test in this repo that touches the dōjō schema stubs the database.
`dojo/src/**/*.spec.ts` mocks the supabase-js client and asserts the payload the
code *sends*, which means **no dōjō test can fail when the schema moves under
it**. That is not a hypothetical: on 2026-08-27 the suite was green at 1328 tests
while two shipped paths were dead —

| path | error |
|---|---|
| `createDojo` → `POST /v1/you/dojos` | `column "org" of relation "tenants" does not exist` |
| every `dojo.identities` read/write | `column "user_id" does not exist` |

— and `admin-data.spec.ts` asserted, in so many words, the exact payload the
database rejects. See spec `docs/spec/dojo/dojo-auth-provisioning.md` §VIII.4.

These tests exist so that class of drift has something it can turn red.

## What belongs here

Assertions that are only true of a **real database**: RLS row visibility,
constraint and enum enforcement, policy grants, function behaviour. Anything
provable against a mock belongs in a unit test next to its module instead — this
harness needs a running Postgres and is correspondingly slower to reach for.

## Running

```bash
database/tests/run.sh                      # local Supabase, the default
DATABASE_URL=postgres://… database/tests/run.sh
```

Exits non-zero on the first failing assertion in any file, and names the file.
`make test-db` runs the same thing.

## Writing a test

One file per concern, under `tests/<schema>/<concern>.sql`. Each file:

- wraps itself in `begin` / `rollback`, so it leaves no rows behind and needs no
  cleanup step that can itself fail;
- raises an exception on a failed assertion — `psql -v ON_ERROR_STOP=1` turns
  that into a non-zero exit, so the runner's status is the real result rather
  than a grep over its output;
- states in a comment what would have to break for it to go red. A test whose
  answer is the same whether the feature works or not is worse than no test.

### Fixture keys must not collide with real data

Prefix every tenant key and slug a fixture inserts with `ztest-`
(`organization/ztest-acme`). The tests run against a **live** database, and
`dojo.tenants.key` is globally unique — a fixture using a plausible name breaks
the moment someone provisions an org with that name.

This is not hypothetical. A fixture using `organization/sensei-hq` passed for
days and then began failing the instant a real GitHub sign-in provisioned that
exact tenant. Rolling back in a transaction protects the database from the test;
it does nothing to protect the test from the database.

To exercise the client-direct path (RLS as a signed-in user):

```sql
set local role authenticated;
set local request.jwt.claims = '{"sub":"<the auth.users id>"}';
```

`auth.uid()` reads `request.jwt.claims ->> 'sub'`, so that pair is what a real
Supabase JWT amounts to as far as a policy is concerned. `reset role` to go back.
Remember the Worker itself connects as `service_role`, which **bypasses RLS** —
a policy can be entirely broken and every app-level test still passes.
