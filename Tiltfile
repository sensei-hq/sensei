# Tiltfile — sensei local dev stack
# Orchestrates: Supabase → dbd setup → sensei MCP server + dashboard
#
# Usage:
#   tilt up          — start everything
#   tilt down        — stop everything
#   tilt up --stream — follow logs inline
#
# Prerequisites: supabase CLI, dbd, bun all on PATH

LOCAL_DB = "postgresql://postgres:postgres@localhost:54322/postgres"
DB_DIR   = "database"
GRANTS   = DB_DIR + "/ddl/grants.ddl"

# ── 1. Supabase ────────────────────────────────────────────────────────────────
# Ensures the sensei supabase project is running.
# If it is already up, skip. If another project is occupying Docker, stop it first.
local_resource(
    "supabase",
    serve_cmd="""
set -e
if supabase status 2>/dev/null | grep -q 'API URL'; then
  echo 'Sensei supabase already running — skipping start'
else
  supabase stop --no-backup 2>/dev/null || true
  supabase start
fi
tail -f /dev/null
""",
    readiness_probe=probe(
        period_secs=5,
        exec=exec_action(["supabase", "status"]),
    ),
    labels=["infra"],
)

# ── 2. Database schema (dbd) ───────────────────────────────────────────────────
# Runs reset → apply → grants whenever DDL files or design.yaml change.
# Must run from the database/ directory because dbd reads design.yaml there.
local_resource(
    "db-apply",
    cmd=" && ".join([
        "cd " + DB_DIR,
        "dbd reset  -d " + LOCAL_DB,
        "dbd apply  -d " + LOCAL_DB,
        "cd ..",
        "psql " + LOCAL_DB + " -f " + GRANTS + " -q",
    ]),
    deps=[DB_DIR + "/ddl", DB_DIR + "/design.yaml"],
    resource_deps=["supabase"],
    labels=["infra"],
)

# ── 3. Seed data (dbd import) ──────────────────────────────────────────────────
# Re-runs when import files change. dbd import also runs from database/.
local_resource(
    "db-seed",
    cmd="cd " + DB_DIR + " && dbd import -d " + LOCAL_DB,
    deps=[DB_DIR + "/import"],
    resource_deps=["db-apply"],
    labels=["infra"],
)

# ── 4. Codebase indexing ───────────────────────────────────────────────────────
# Populates sensei.symbols and sensei.scan_state via the engine indexer.
# Re-runs on any source file change (broad dep — Tilt will debounce).
# create-test-user.sh links the auth user to the seeded core.accounts row.
local_resource(
    "db-index",
    cmd=" && ".join([
        "bash scripts/create-test-user.sh",
        "bun packages/server/src/e2e-index.ts",
    ]),
    resource_deps=["db-seed"],
    labels=["infra"],
)

# ── 5. MCP server ──────────────────────────────────────────────────────────────
# Starts the sensei MCP/HTTP server with hot reload via bun --watch.
local_resource(
    "mcp-server",
    serve_cmd="bun run --watch packages/server/src/index.ts",
    resource_deps=["db-index"],
    labels=["app"],
)

# ── 5. Dashboard (SvelteKit dev server) ────────────────────────────────────────
local_resource(
    "dashboard",
    serve_cmd="bun --filter=dashboard run dev",
    resource_deps=["supabase"],
    labels=["app"],
)
