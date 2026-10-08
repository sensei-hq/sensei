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
test. The exemption is by MODULE, not by filename, so moving a statement above a
`#[cfg(test)]` marker to dodge the gate does not work.

The marker is the first line starting with `#[cfg(test)]` at column 0, matching
how the repo's own inventories counted this population. A file whose whole
contents are tests but which carries no such marker — `db/pg_store/tests.rs` and
the `*_tests.rs` siblings — is exempted by name, because its module declaration
lives elsewhere.
"""

import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SRC = ROOT / "crates/senseid/src"
LAYER = "db/pg_store/"
# Whole-file test modules, declared `#[cfg(test)] mod <name>;` from mod.rs.
TEST_FILE_SUFFIXES = ("/tests.rs", "_tests.rs", "/test_support.rs")
CALL = re.compile(r"sqlx_core::query(_as)?::query(_as)?\b")


def first_test_module(path: pathlib.Path) -> int:
    """Line number of the file's `#[cfg(test)]` marker, or a line past the end."""
    for n, line in enumerate(path.read_text().splitlines(), 1):
        if line.startswith("#[cfg(test)]"):
            return n
    return 1 << 30


def main() -> int:
    hits = subprocess.run(
        ["rg", "-n", "--no-ignore", "-g", "!target", "sqlx_core::query",
         str(SRC), "--type", "rust"],
        capture_output=True, text=True,
    ).stdout.splitlines()

    offences = []
    cache: dict[str, int] = {}
    for line in hits:
        path_s, lineno, text = line.split(":", 2)
        rel = str(pathlib.Path(path_s).resolve().relative_to(ROOT))
        if LAYER in rel or rel.endswith(TEST_FILE_SUFFIXES):
            continue
        # A type position (`QueryAs<...>` in a signature) is not a call site.
        if not CALL.search(text):
            continue
        if path_s not in cache:
            cache[path_s] = first_test_module(pathlib.Path(path_s))
        if int(lineno) >= cache[path_s]:
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
