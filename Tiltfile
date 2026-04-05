# Tiltfile — sensei local dev stack
# Orchestrates: Supabase → dbd setup → sensei MCP server + dashboard
#
# Usage:
#   tilt up          — start everything
#   tilt down        — stop everything (keeps Supabase containers)
#   tilt up --stream — follow logs inline
#
# Prerequisites: supabase CLI, dbd, bun all on PATH

DATABASE_URL = "postgresql://postgres:postgres@127.0.0.1:54322/postgres"
DB_DIR       = "database"

# ── 1. Supabase ────────────────────────────────────────────────────────────────
# supabase start launches the local stack and exits when all services are ready.
# Re-running when already started is a no-op (exits 0).
local_resource(
    "supabase",
    cmd="supabase start",
    links=[
        link("http://127.0.0.1:54323", "Studio"),
        link("http://127.0.0.1:54321", "Supabase API"),
        link("http://127.0.0.1:54324", "Email (Inbucket)"),
    ],
    labels=["infra"],
)

# ── 2. Database setup (dbd) ───────────────────────────────────────────────────
# reset → apply → grants → import, all from the database/ directory.
# Re-runs automatically when DDL files, import data, or design.yaml change.
local_resource(
    "db:setup",
    cmd=(
        "cd " + DB_DIR + " && " +
        "DATABASE_URL={url} dbd reset && " +
        "DATABASE_URL={url} dbd apply && " +
        "DATABASE_URL={url} dbd grants && " +
        "DATABASE_URL={url} dbd import"
    ).format(url=DATABASE_URL),
    deps=[
        DB_DIR + "/ddl",
        DB_DIR + "/import",
        DB_DIR + "/design.yaml",
    ],
    resource_deps=["supabase"],
    labels=["infra"],
)

# ── 3. Codebase indexing ───────────────────────────────────────────────────────
# Populates sensei.symbols and sensei.scan_state via the engine indexer.
# create-test-user.sh links the auth user to the seeded core.accounts row.
local_resource(
    "db:index",
    cmd=" && ".join([
        "bash scripts/create-test-user.sh",
        "bun packages/server/src/e2e-index.ts",
    ]),
    resource_deps=["db:setup"],
    labels=["infra"],
)

# ── 4. MCP server ──────────────────────────────────────────────────────────────
# Starts the sensei MCP/HTTP server with hot reload via bun --watch.
local_resource(
    "mcp-server",
    serve_cmd="bun run --watch packages/server/src/index.ts",
    resource_deps=["db:index"],
    links=[
        link("http://localhost:3001", "MCP Server"),
    ],
    labels=["app"],
)

# ── 5. Dashboard (SvelteKit / Vite on :5173) ───────────────────────────────────
local_resource(
    "dashboard",
    serve_cmd="cd apps/dashboard && bun run dev",
    resource_deps=["supabase"],
    links=[
        link("http://localhost:5173",          "App"),
        link("http://localhost:5173/mockups",  "Mockups"),
    ],
    labels=["app"],
)
