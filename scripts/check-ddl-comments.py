#!/usr/bin/env python3
"""Keep prose out of the column list.

A `create table` column list is a DECLARATION. When paragraphs of rationale sit
between two columns, the shape of the table — which is the thing a reader opened
the file for — is buried, and the prose is invisible to every consumer that is
not a human reading this exact file.

There are two better homes and this repo already uses both:

  `comment on column <table>.<col>`   what the column MEANS. Lands in the
                                      database catalog, so `\\d+` and any tool
                                      that reads `pg_description` can see it.
  docs/database/<schema>.md           design history: what was measured, what
                                      was tried and failed, what not to change.
                                      Prose that is ABOUT the decision rather
                                      than about the column.

A SINGLE line is left alone. A one-line `-- skill only` group label or a `── … ──`
divider is formatting, not prose, and relocating it would lose the grouping it
exists to show. Two consecutive lines is where narrative starts.

Usage:
    check-ddl-comments.py            check the tree, exit 1 on findings
    check-ddl-comments.py --list     one finding per line, for scripting
    check-ddl-comments.py --self-test
"""
from __future__ import annotations

import pathlib
import re
import subprocess
import sys

ROOT = "database/ddl/table"
CREATE = re.compile(r"^create table[^(]*\(", re.I | re.M)


def column_list_span(text: str) -> tuple[int, int] | None:
    """Line indices [start, end) of the create-table column list.

    Found by matching parentheses from the `create table (` rather than by a
    regex for `^);` — a column list can legitimately contain a parenthesised
    default or check, and a line-anchored terminator would stop at the wrong one.
    """
    m = CREATE.search(text)
    if not m:
        return None
    depth = 0
    started = False
    for i, ch in enumerate(text[m.start():], start=m.start()):
        if ch == "(":
            depth += 1
            started = True
        elif ch == ")":
            depth -= 1
            if started and depth == 0:
                first = text.count("\n", 0, m.end())
                last = text.count("\n", 0, i)
                return first, last
    return None


def findings(path: pathlib.Path) -> list[tuple[int, int, str]]:
    """(line_number, block_length, first_line) for every 2+ line comment run."""
    text = path.read_text(errors="replace")
    span = column_list_span(text)
    if not span:
        return []
    start, end = span
    lines = text.split("\n")
    out = []
    run: list[str] = []
    run_at = 0
    for idx in range(start, min(end, len(lines))):
        stripped = lines[idx].strip()
        if stripped.startswith("--"):
            if not run:
                run_at = idx + 1
            run.append(stripped[2:].strip())
        else:
            if len(run) >= 2:
                out.append((run_at, len(run), run[0]))
            run = []
    if len(run) >= 2:
        out.append((run_at, len(run), run[0]))
    return out


def ddl_files() -> list[pathlib.Path]:
    listed = subprocess.run(["git", "ls-files", ROOT], capture_output=True, text=True).stdout.split()
    return [pathlib.Path(f) for f in listed if f.endswith(".ddl")]


def self_test() -> int:
    import tempfile

    cases = [
        ("one line is fine", """create table if not exists t (
  id uuid primary key
  -- skill only
, a text
);""", 0),
        ("two lines is prose", """create table if not exists t (
  id uuid primary key
  -- this is a narrative that runs
  -- across two lines
, a text
);""", 1),
        ("prose before the close", """create table if not exists t (
  id uuid primary key
  -- trailing narrative
  -- second line
);""", 1),
        ("comments after the list are fine", """create table if not exists t (
  id uuid primary key
);
-- a long explanation
-- that lives below the declaration
comment on column t.id is 'x';""", 0),
        ("a parenthesised default does not end the list", """create table if not exists t (
  id uuid primary key default gen_random_uuid()
, b numeric(3,2)
  -- narrative that would be missed
  -- if the scan stopped at the first close paren
, c text
);""", 1),
    ]
    passed = 0
    for name, body, want in cases:
        with tempfile.NamedTemporaryFile("w", suffix=".ddl", delete=False) as fh:
            fh.write(body)
            p = pathlib.Path(fh.name)
        got = len(findings(p))
        p.unlink()
        if got == want:
            passed += 1
            print(f"  ok    {name}")
        else:
            print(f"  FAIL  {name}: found {got}, expected {want}")
    print(f"  {passed}/{len(cases)} passed")
    return 0 if passed == len(cases) else 1


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    as_list = "--list" in sys.argv

    total_blocks = total_lines = 0
    per_file = []
    for f in ddl_files():
        found = findings(f)
        if not found:
            continue
        per_file.append((sum(n for _, n, _ in found), f, found))
        total_blocks += len(found)
        total_lines += sum(n for _, n, _ in found)

    if as_list:
        for _, f, found in per_file:
            for line, n, first in found:
                print(f"{f}:{line}:{n}:{first[:70]}")
        return 1 if per_file else 0

    if not per_file:
        print("check-ddl-comments: no prose between columns in any table DDL")
        return 0

    per_file.sort(reverse=True)
    print(f"check-ddl-comments: {total_blocks} prose blocks ({total_lines} lines) "
          f"inside a column list, across {len(per_file)} files\n")
    for n, f, found in per_file:
        print(f"  {n:>4} lines  {f}")
        for line, ln, first in found[:3]:
            print(f"              :{line} ({ln} lines) {first[:64]}")
        if len(found) > 3:
            print(f"              … and {len(found) - 3} more")
    print("\nMove each block to one of:")
    print("  comment on column <table>.<col>   what the column MEANS")
    print("  docs/database/<schema>.md         why it is that way")
    return 1


if __name__ == "__main__":
    sys.exit(main())
