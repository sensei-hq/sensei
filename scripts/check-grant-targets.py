#!/usr/bin/env python3
"""Every role the DDL grants to must be a role the DDL itself creates.

WHY. A `GRANT` resolves its grantee at apply time. Twelve files in the `sensei`
and `activity` schemas grant to Supabase's `anon` / `authenticated` /
`service_role`, and nothing in the tree created them — so on a Postgres that had
never hosted Supabase, the first such file aborted the entire deploy with
`role "authenticated" does not exist` and a first install could not provision its
database (issue #196). It was invisible on every machine that already had the
roles.

`database/apply/before/roles.sql` now creates them. This check is the regression
gate, and it is STATIC on purpose: the CI provision against a clean Postgres also
catches this, but only after a full ~20-minute deploy and only for grantees in
the `default` scope. This runs in seconds, needs no database, and covers every
scope — including `dojo`, whose files a daemon-scoped provision never applies.

It is a real gate, not a restatement: adding `grant select on t to some_new_role`
without creating `some_new_role` turns it red.

Exit 0 when every grantee is accounted for, 1 otherwise.
"""

import re
import sys
from pathlib import Path

# Roles Postgres itself defines, which no project needs to create. `public` is
# the implicit pseudo-role every grant can name.
BUILTIN_ROLES = {
    "public",
    "postgres",
    "pg_read_all_data",
    "pg_write_all_data",
    "pg_monitor",
    "pg_read_all_settings",
    "pg_read_all_stats",
    "pg_stat_scan_tables",
    "pg_signal_backend",
    "pg_checkpoint",
    "pg_maintain",
    "pg_use_reserved_connections",
    "pg_create_subscription",
    "current_user",
    "session_user",
    "current_role",
}

# `grant <privs> on <thing> to a, b, c` / `revoke … from a, b`. Captures the
# comma-separated grantee list, which is everything up to the statement end or a
# trailing `with grant option`.
GRANTEES = re.compile(
    r"\b(?:grant|revoke)\b.*?\b(?:to|from)\s+([^;]+?)(?:\s+with\s+grant\s+option)?\s*;",
    re.IGNORECASE | re.DOTALL,
)

# `create policy … to <roles>` resolves its roles at apply time exactly as a
# GRANT does, so a policy naming an absent role aborts the deploy the same way.
# This is not hypothetical: `anon` is named ONLY here, never as a grant target,
# so a grant-only check reported 2 of the 3 missing roles and called that a
# complete answer. The `to` clause ends at `using`, `with check`, or the
# statement.
POLICY_ROLES = re.compile(
    r"\bcreate\s+policy\b.*?\bto\s+([^;]+?)(?:\s+using\b|\s+with\s+check\b|\s*;)",
    re.IGNORECASE | re.DOTALL,
)

# `create role "x"` / `create role x`, and the `format('create role %I', r)`
# shape the roles hook uses — there the created names come from its array
# literal, so read those too.
CREATE_ROLE = re.compile(r"\bcreate\s+role\s+\"?([A-Za-z_][\w$]*)\"?", re.IGNORECASE)
ROLE_ARRAY = re.compile(r"array\s*\[([^\]]+)\]", re.IGNORECASE)


def strip_sql_comments(sql: str) -> str:
    """Drop `--` line comments and `/* */` blocks so commented-out grants and
    prose about roles cannot be mistaken for statements. Without this, a comment
    reading "no client authenticated/anon grant" parses as a grantee list."""
    sql = re.sub(r"/\*.*?\*/", " ", sql, flags=re.DOTALL)
    return re.sub(r"--[^\n]*", " ", sql)


def grantees_in(sql: str) -> set[str]:
    """Every role name this SQL requires to already exist — grant/revoke targets
    and the roles a policy applies to."""
    found: set[str] = set()
    for pattern in (GRANTEES, POLICY_ROLES):
        for match in pattern.findall(sql):
            for raw in match.split(","):
                name = raw.strip().strip('"').strip()
                # Skip anything that is not a bare identifier: `group x`, a role
                # expression, or a fragment of a statement we mis-split.
                if re.fullmatch(r"[A-Za-z_][\w$]*", name or ""):
                    found.add(name.lower())
    return found


def created_roles(sql: str) -> set[str]:
    made = {m.lower() for m in CREATE_ROLE.findall(sql)}
    # A `format('create role %I', r)` loop names its roles in an array literal.
    if "create role" in sql.lower():
        for arr in ROLE_ARRAY.findall(sql):
            for item in arr.split(","):
                name = item.strip().strip("'\"").strip()
                if re.fullmatch(r"[A-Za-z_][\w$]*", name or ""):
                    made.add(name.lower())
    return made


def main() -> int:
    root = Path(__file__).resolve().parent.parent / "database"
    if not root.is_dir():
        print(f"check-grant-targets: no database tree at {root}", file=sys.stderr)
        return 1

    # Everything that can issue a GRANT: the DDL, the RLS policies, and the
    # apply hooks. NOT `tests/` (it asserts against a deployed database) and NOT
    # `import/` (seed data).
    sources = sorted(
        p
        for d in ("ddl", "policies", "apply")
        for p in (root / d).rglob("*.sql")
    ) + sorted((root / "ddl").rglob("*.ddl"))

    wanted: dict[str, list[str]] = {}
    created: set[str] = set()
    for path in sources:
        sql = strip_sql_comments(path.read_text(errors="replace"))
        created |= created_roles(sql)
        for role in grantees_in(sql):
            wanted.setdefault(role, []).append(str(path.relative_to(root)))

    known = created | BUILTIN_ROLES
    missing = {r: files for r, files in wanted.items() if r not in known}

    print(
        f"check-grant-targets: {len(sources)} files, "
        f"{len(wanted)} distinct grantee(s), {len(created)} created by the schema."
    )
    if not missing:
        print("check-grant-targets: every grantee is created by the schema or built in.")
        return 0

    print(
        f"\ncheck-grant-targets: {len(missing)} grantee(s) that NOTHING creates —"
        " a deploy against a database lacking them aborts on the first grant:",
        file=sys.stderr,
    )
    for role, files in sorted(missing.items()):
        print(f"    {role}  ({len(files)} file(s), e.g. {files[0]})", file=sys.stderr)
    print(
        "\nCreate them in database/apply/before/roles.sql, which runs before the"
        " first entity is written.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
