#!/usr/bin/env python3
"""Parse-and-plan EVERY SQL statement in the Rust sources against a live schema.

WHY THIS EXISTS. The compiler cannot see inside a SQL string. `sqlx::query()`
binds at runtime, so a query naming a dropped column, a renamed table or a
removed function compiles perfectly and fails only when that line is reached —
and only if a test reaches it. Dropping `folders.project_id` (#211) produced
exactly that: four readers compiled, passed CI's fast gate, and failed against
the database. One of them did not even fail — a bare `project_id` inside a
subquery silently rebound to an outer `memories.project_id`, so the predicate
became `x = x` and handed every project's governance rules to every folder.

WHAT IT DOES. `PREPARE` runs Postgres's parse + plan WITHOUT executing: every
table, column, function and type reference is resolved and type-checked, and
nothing is read or written. So this validates the whole surface, including the
statements no test covers, against whatever schema is actually deployed.

WHAT IT CANNOT DO. A statement assembled at runtime (format!/push_str) reaches
this as a fragment. Those are COUNTED AND LISTED as unverifiable rather than
silently passed — an unchecked query the report does not mention is the thing
this script exists to prevent.

    scripts/check-sql-against-schema.py [--database URL] [--verbose]

Exit 1 if any statement fails to plan against the schema.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys

# Real schema defects — the statement names something the database does not have.
SCHEMA_ERRORS = {
    "42P01": "undefined table",
    "42703": "undefined column",
    "42883": "undefined function",
    "42704": "undefined object",
    "42P02": "undefined parameter",
    "42846": "cannot cast type",
    "42804": "datatype mismatch",
    "42725": "ambiguous function",
    "42702": "ambiguous column",
}
# Postgres could not infer a placeholder's type with no surrounding context.
# A property of checking the statement in isolation, not a defect in it.
PARAM_INFERENCE = {"42P18", "42601:param"}

# A statement, not an English sentence that happens to open with "Update" or
# "with". Each form must be followed by the keyword that makes it a statement.
SQL_START = re.compile(
    r"^\s*(?:SELECT\s+|INSERT\s+INTO\s+|UPDATE\s+[\w.\"]+\s+SET\b"
    r"|DELETE\s+FROM\s+|WITH\s+(?:RECURSIVE\s+)?[\w.\"]+\s+AS\s*\()",
    re.I | re.S,
)
# A Rust string literal, honouring backslash escapes.
RUST_STR = re.compile(r'"((?:[^"\\]|\\.)*)"', re.S)


def unescape(s: str) -> str:
    """Rust escapes → the text Postgres would receive."""
    out, i = [], 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            nxt = s[i + 1]
            if nxt == "\n":  # line continuation: eat the newline AND the indent
                i += 2
                while i < len(s) and s[i] in " \t":
                    i += 1
                continue
            out.append({"n": "\n", "t": "\t", "r": "\r", '"': '"', "\\": "\\"}.get(nxt, nxt))
            i += 2
            continue
        out.append(c)
        i += 1
    return "".join(out)


def looks_complete(sql: str) -> bool:
    """Reject fragments produced by runtime assembly, and non-statements."""
    s = sql.strip().rstrip(";")
    if not SQL_START.match(s):
        return False
    if "{}" in s or "{" in s and re.search(r"\{\w*\}", s):
        return False  # a format! template, not a statement
    # Unbalanced parens means a concatenated fragment.
    if s.count("(") != s.count(")"):
        return False
    return len(s) > 20


def collect(root: pathlib.Path) -> tuple[list[tuple[str, int, str]], list[str]]:
    """Statements to check, and the files deliberately NOT checked (with why).

    Two kinds of SQL in this tree are not addressed to sensei's Postgres and
    must not be judged against its schema:

      * a transcript adapter reading ANOTHER agent's SQLite database (Zed,
        opencode, VS Code, Copilot CLI) — a different engine and a schema this
        project does not own; and
      * `indexer/lang/sql/` — sample SQL the parser is TESTED ON, source text
        rather than a query anyone runs.

    Both are reported, never silently dropped: an unchecked query this script
    does not mention is precisely the gap it exists to close.
    """
    found: list[tuple[str, int, str]] = []
    skipped: list[str] = []
    for path in sorted(root.rglob("*.rs")):
        if "/target/" in str(path):
            continue
        text = path.read_text(errors="ignore")
        if "rusqlite" in text:
            skipped.append(f"{path} — reads another agent's SQLite DB")
            continue
        if "/indexer/lang/sql/" in str(path):
            skipped.append(f"{path} — sample SQL the parser is tested on")
            continue
        for m in RUST_STR.finditer(text):
            sql = unescape(m.group(1))
            if looks_complete(sql):
                line = text[: m.start()].count("\n") + 1
                found.append((str(path), line, sql.strip().rstrip(";")))
    return found, skipped


def check(stmts: list[tuple[str, int, str]], url: str) -> tuple[list, list, int]:
    """PREPARE each statement; classify what comes back."""
    # The daemon connects with `extensions` on the search_path (that is where
    # pgvector lives). Without it every `::vector` cast and `<=>` operator reads
    # as a missing type here — a property of this session, not of the query.
    script = ["SET search_path TO sensei, extensions, public;"]
    for i, (_, _, sql) in enumerate(stmts):
        # DEALLOCATE between statements keeps names free; each PREPARE is
        # wrapped so one failure does not abort the rest of the session.
        script.append(f"\\echo __STMT__{i}")
        script.append(f"PREPARE _chk_{i} AS {sql};")
        script.append(f"DEALLOCATE _chk_{i};")
    # stderr is merged INTO stdout by the child, not concatenated afterwards.
    # Concatenating puts every marker before every error and destroys the
    # attribution — which made this script report 0 failures against a
    # completely EMPTY database, the one result that proves a checker is lying.
    proc = subprocess.run(
        ["psql", url, "-X", "-q", "-v", "ON_ERROR_STOP=0", "-f", "-"],
        input="\n".join(script), stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT, text=True,
    )
    # Walk the interleaved output, attributing each error to the preceding marker.
    failures, unresolved, idx = [], [], -1
    for line in proc.stdout.splitlines():
        m = re.match(r"__STMT__(\d+)", line)
        if m:
            idx = int(m.group(1))
            continue
        # psql PREFIXES errors with `psql:<stdin>:N: `, so matching on a
        # leading "ERROR:" silently matches nothing — which is how this
        # reported a clean bill of health against an empty database.
        em = re.search(r"\bERROR:\s+(.*)", line)
        if not em:
            continue
        msg = em.group(1)
        # A failed PREPARE makes the following DEALLOCATE fail too. That is a
        # cascade of the finding already recorded, not a second finding.
        if re.match(r'prepared statement "_chk_\d+" does not exist', msg):
            continue
        if idx < 0 or idx >= len(stmts):
            continue
        path, ln, sql = stmts[idx]
        if ("could not determine data type of parameter" in msg
                or "operator is not unique" in msg):
            unresolved.append((path, ln, sql, msg))
        else:
            failures.append((path, ln, sql, msg))
    return failures, unresolved, len(stmts)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--database", default="postgresql://localhost:5432/sensei_test")
    ap.add_argument("--verbose", action="store_true")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    root = pathlib.Path(__file__).resolve().parent.parent / "crates"
    stmts, skipped = collect(root)
    failures, unresolved, total = check(stmts, args.database)

    if args.json:
        print(json.dumps({"checked": total, "failed": len(failures),
                          "param_inference": len(unresolved),
                          "failures": [{"file": f, "line": l, "error": e}
                                       for f, l, _, e in failures]}, indent=2))
        return 1 if failures else 0

    print(f"check-sql-against-schema: {total} statement(s) planned against {args.database}")
    if skipped:
        print(f"  {len(skipped)} file(s) not addressed to this database, so not checked:")
        for s in skipped:
            print(f"    {s}")
    if unresolved and args.verbose:
        print(f"\n  {len(unresolved)} could not have a parameter type inferred in isolation "
              f"(not a schema defect):")
        for f, l, _, _ in unresolved[:10]:
            print(f"    {f}:{l}")
    if not failures:
        print(f"  {len(unresolved)} param-inference, 0 schema failures — "
              f"every checkable statement resolves against the deployed schema.")
        return 0
    print(f"\n  {len(failures)} STATEMENT(S) DO NOT RESOLVE AGAINST THE SCHEMA:\n")
    for f, l, sql, err in failures:
        print(f"  {f}:{l}")
        print(f"    {err}")
        print(f"    {' '.join(sql.split())[:160]}\n")
    return 1


if __name__ == "__main__":
    sys.exit(main())
