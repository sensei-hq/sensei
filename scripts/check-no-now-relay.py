#!/usr/bin/env python3
"""No `relay` DDL file calls `now()` — `current_timestamp` only.

WHY. dbd 0.27.8 resolved the function call `now()` to the VIEW `relay.now`,
so every `default now()` in `ddl/table/relay/*` plus the window predicate in
`ddl/view/relay/now.ddl` read as a dependency on a view that depends back —
"Cyclic dependency detected" on all 6 tables and both views, and apply,
reconcile and deploy refused to run (relay impl r1 blocker B1). dbd 0.27.9
fixes the mis-resolution upstream (sensei-hq/dbd#112), but the daemon deploys
through its own embedded `dbd-core`, which may lag the CLI — so the spelling
is pinned here instead of relying on any one dbd to fail.

It is a real gate, not a restatement: writing `default now()` in a relay
table file turns it red. Comments and string literals don't count (the
trap-closing comment in `now.ddl` names the function deliberately).

Scope is `relay` only: `now()` stays idiomatic everywhere else (the dojo and
sensei DDL use it, as do the SQL tests, which dbd never parses).

Exit 0 when no relay DDL file calls `now()`, 1 otherwise. `--test` runs the
inline self-test instead.
"""

import re
import sys
from pathlib import Path

NOW_CALL = re.compile(r"\bnow\s*\(", re.IGNORECASE)
BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)
STRING = re.compile(r"'(?:[^']|'')*'", re.DOTALL)
LINE_COMMENT = re.compile(r"--.*$")


def code_only(text):
    """Strip block comments, string literals, then line comments."""
    text = BLOCK_COMMENT.sub("", text)
    text = STRING.sub("''", text)
    return "\n".join(LINE_COMMENT.sub("", line) for line in text.split("\n"))


def violations(path):
    """(line, snippet) hits calling now() in real code, or []."""
    hits = []
    for i, line in enumerate(code_only(path.read_text()).split("\n"), 1):
        if NOW_CALL.search(line):
            hits.append((i, line.strip()[:80]))
    return hits


def relay_ddl_files(root):
    db = root / "database"
    found = []
    for rel in [
        "ddl/enum/relay",
        "ddl/table/relay",
        "ddl/view/relay",
        "policies/relay",
    ]:
        d = db / rel
        if d.is_dir():
            found += sorted(d.glob("*.ddl")) + sorted(d.glob("*.sql"))
    for rel in [
        "ddl/table/staging/finding_classes.ddl",
        "ddl/procedure/staging/import_finding_classes.ddl",
        "apply/after/relay_api_grants.sql",
    ]:
        p = db / rel
        if p.is_file():
            found.append(p)
    return found


def self_test():
    cases = [
        ("created_at timestamptz not null default now(),", True),
        (">= date_trunc('day', current_timestamp);", False),
        ("-- current_timestamp, never now(): dbd resolves now() to this view.", False),
        ("/* a now() in a block comment */ select 1;", False),
        ("comment is 'wrapped now() in a string';", False),
        ("select NOW() from t;", True),
        ("select now () from t;", True),
        ("select snow(x) from t;", False),
        ("select renowned from t;", False),
    ]
    failed = 0
    for text, want in cases:
        got = bool(NOW_CALL.search(code_only(text)))
        if got != want:
            print(f"self-test: {text!r} -> {got}, want {want}", file=sys.stderr)
            failed += 1
    if failed:
        print(f"check-no-now-relay: self-test {failed} case(s) red", file=sys.stderr)
        return 1
    print("check-no-now-relay: self-test green")
    return 0


def main():
    root = Path(__file__).resolve().parent.parent
    files = relay_ddl_files(root)
    if not files:
        print("check-no-now-relay: no relay DDL files found — scope broken?", file=sys.stderr)
        return 1
    bad = {}
    for path in files:
        hits = violations(path)
        if hits:
            bad[path.relative_to(root)] = hits
    print(f"check-no-now-relay: {len(files)} files scanned.")
    if not bad:
        print("check-no-now-relay: no now() calls in relay DDL.")
        return 0
    print(
        "\ncheck-no-now-relay: now() calls that dbd may resolve to relay.now:",
        file=sys.stderr,
    )
    for path, hits in sorted(bad.items()):
        for line, snippet in hits:
            print(f"    {path}:{line}: {snippet}", file=sys.stderr)
    print("\nUse current_timestamp — see the B1 comment in ddl/view/relay/now.ddl.", file=sys.stderr)
    return 1


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--test":
        sys.exit(self_test())
    sys.exit(main())
