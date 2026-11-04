#!/usr/bin/env python3
"""Refuse production SQL outside the persistence layer (#227).

`crates/senseid/src/db/pg_store/` IS the persistence layer. Thirty production
statements lived outside it — the metric computers, the gateway catalogue read,
one federation lookup — and an ADR briefly defended that as a deliberate
exception. It was superseded the same day for a reason this script exists to
answer: a documented exception has no enforcement. Nothing failed when the next
metric added the thirty-first query, and nothing distinguished "deliberate" from
"nobody noticed".

WHAT IT CHECKS. Every `sqlx_core::query*` call site in `crates/senseid/src` that
is NOT under `db/pg_store/` and NOT inside a `#[cfg(test)]` module.

WHY THE TEST EXEMPTION IS NOT A HOLE. A fixture writing rows directly is
modelling a state production reaches some other way; it is not a data-access
path a schema change can silently break, because a broken fixture fails its own
test. The exemption is by ITEM, not by filename or position, so moving a
statement above a `#[cfg(test)]` marker to dodge the gate does not work.

What counts as test code — each item under `#[cfg(test)]` (an inline module or a
test-only fn), and whole-file test modules such as `db/pg_store/tests.rs` and the
`*_tests.rs` siblings whose declaration lives elsewhere — is defined once in
`scripts/_rust_scan.py`, shared with `check-no-fabrication.py`.
"""

import pathlib
import re
import subprocess
import sys

sys.dont_write_bytecode = True  # no __pycache__ beside the scripts
from _rust_scan import TestCode  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "crates/senseid/src"
LAYER = "db/pg_store/"
CALL = re.compile(r"sqlx_core::query(_as)?::query(_as)?\b")


def main() -> int:
    hits = subprocess.run(
        ["rg", "-n", "--no-ignore", "-g", "!target", "sqlx_core::query",
         str(SRC), "--type", "rust"],
        capture_output=True, text=True,
    ).stdout.splitlines()

    offences = []
    tests = TestCode(ROOT, sorted(SRC.rglob("*.rs")))
    for line in hits:
        path_s, lineno, text = line.split(":", 2)
        rel = str(pathlib.Path(path_s).resolve().relative_to(ROOT))
        if LAYER in rel:
            continue
        # A type position (`QueryAs<...>` in a signature) is not a call site.
        if not CALL.search(text):
            continue
        if tests.is_test_line(pathlib.Path(path_s), int(lineno)):
            continue
        offences.append((rel, lineno, text.strip()))

    if not offences:
        print("check-sql-in-layer: no production SQL outside db/pg_store/ — the layer holds.")
        return 0

    print(f"check-sql-in-layer: {len(offences)} production statement(s) outside the "
          f"persistence layer.\n")
    for rel, lineno, text in offences:
        print(f"  {rel}:{lineno}")
        print(f"    {text[:110]}")
    print(
        "\nMove each into crates/senseid/src/db/pg_store/ and call it from here.\n"
        "A read that splices a window or a filter should take the WINDOW as a\n"
        "parameter and build the predicate inside the layer — a layer method whose\n"
        "SQL still arrives from outside it is the same problem wearing a different\n"
        "hat. See db/pg_store/metric_reads.rs for the shape, and #227 for why this\n"
        "is a gate rather than a convention."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
