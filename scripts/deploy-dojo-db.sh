#!/usr/bin/env bash
#
# Deploy the dōjō schema to $DATABASE_URL — run from database/.
#
# ONE path for production (release.yml's deploy-dojo) and for CI (a fresh
# Supabase before the dōjō's access-rule tests), so every schema change
# rehearses the exact steps production takes. Tested with a fake dbd by
# crates/bootstrap/tests/deploy_dojo_db_script.rs.
set -euo pipefail

# Refuse to "succeed" without deploying anything.
if [ -z "${DATABASE_URL:-}" ]; then
    echo "DATABASE_URL is not set — refusing to report a deploy that applied nothing" >&2
    exit 1
fi

# THE critical guard. dbd drops a file it cannot parse from the entity set
# SILENTLY and still reports success — how dojo.set_pack_adoption and
# dojo.can_read_repository_metric were never created on any database — and
# `dbd inspect` still exits 0 when a file fails to parse. So grep the output.
inspect_log=$(mktemp)
trap 'rm -f "$inspect_log"' EXIT
dbd inspect 2>&1 | tee "$inspect_log"
if grep -q "Parse error" "$inspect_log"; then
    echo "::error::DDL failed to parse — those entities would be skipped silently" >&2
    exit 1
fi

# The plan, printed before anything is applied, so the log records what this
# deploy changed. Never fails: a fresh or drifted database legitimately differs.
dbd diff --scope dojo || true

# Which tool applies a change depends on whether the project is RELEASED.
# Pre-release: `reconcile --allow-destructive`, unattended — the user's decision
# (2026-10-20): the dōjō has not released v1, so its data is expendable and a
# reshape may drop columns and rows. At v1 (on the user's explicit request) the
# reconcile goes and `dbd deploy` migrates from the committed snapshot.
if grep -qE '^[[:space:]]*released:[[:space:]]*true' design.yaml; then
    echo "Project is released - the deploy step migrates from the committed snapshot."
    dbd migrate --status
else
    dbd reconcile --scope dojo --allow-destructive
fi

# Seed data, RLS policies and after-scripts (the relay tables' Realtime
# publication); post-release, also the migration. Idempotent: every import is
# modified_at-guarded.
dbd deploy --scope dojo

# The proof the deploy did something: 2 when differences remain. This also
# catches a released DDL change whose snapshot was never taken — it deploys
# NOTHING while reporting success, and the diff still shows the change.
dbd diff --scope dojo --exit-code
