#!/usr/bin/env python3
"""Move prose out of a column list, into the home that fits it.

Two destinations, and the split is the whole point:

  CATALOG   `comment on column <table>.<col>` — what the column MEANS. It ends
            up in `pg_description`, so `\\d+` and anything reading the catalog
            can see it, instead of being visible only to someone opening this
            one file.
  MARKDOWN  `docs/database/<schema>.md` — design HISTORY. What was measured,
            what was tried and failed, what must not be changed. Prose about
            the DECISION rather than about the column. A catalog description
            that says "do not optimise this" is describing the maintainer, not
            the data.

Run `--plan` first and read it. The classification below is a heuristic over
prose, and a heuristic that silently rewrites 39 files is not something to
trust unread. `--apply` only ever acts on a plan file.

Usage:
    relocate-ddl-comments.py --plan            write /tmp/ddl-plan.json
    relocate-ddl-comments.py --plan --only edges.ddl
    relocate-ddl-comments.py --apply           apply /tmp/ddl-plan.json
"""
from __future__ import annotations

import json
import pathlib
import re
import subprocess
import sys

PLAN = pathlib.Path("/tmp/ddl-plan.json")
CREATE = re.compile(r"^create table(?:\s+if\s+not\s+exists)?\s+([A-Za-z0-9_.\"]+)\s*\(", re.I | re.M)

# Prose that is about the DECISION, not about the column. Dates and counts are
# the strongest signal: a measurement is a record of how something was learned.
HISTORY = re.compile(
    r"\b(measured|proven|refuted|20\d\d-\d\d-\d\d|\bdo not\b|don't|must not|"
    r"used to|previously|earlier version|first version|did not work|"
    r"before this|accumulated|regression|audit|deliberately not|"
    r"R\d+(\.\d+)?\b|S\d+\b|#\d{2,}|\d{1,3}(,\d{3})+)\b", re.I)

CONSTRAINT = re.compile(r"^\s*,?\s*(constraint|primary\s+key|unique|check|foreign\s+key|exclude)\b", re.I)
COLUMN = re.compile(r"^\s*,?\s*\"?([a-z_][a-z0-9_]*)\"?\s+\S")


def column_list_span(text: str):
    m = CREATE.search(text)
    if not m:
        return None, None
    depth, started = 0, False
    for i, ch in enumerate(text[m.start():], start=m.start()):
        if ch == "(":
            depth += 1
            started = True
        elif ch == ")":
            depth -= 1
            if started and depth == 0:
                return (text.count("\n", 0, m.end()), text.count("\n", 0, i)), m.group(1)
    return None, None


def plan_file(path: pathlib.Path) -> list[dict]:
    text = path.read_text(errors="replace")
    span, table = column_list_span(text)
    if not span:
        return []
    start, end = span
    lines = text.split("\n")
    schema = path.parent.name
    table_short = table.split(".")[-1].strip('"')

    out, run, run_at = [], [], 0
    last_col = None
    for idx in range(start, min(end + 1, len(lines))):
        raw = lines[idx]
        s = raw.strip()
        if s.startswith("--"):
            if not run:
                run_at = idx
            run.append(s[2:].strip())
            continue
        if run:
            if len(run) >= 2:
                target, why = None, None
                if CONSTRAINT.match(raw):
                    why = "precedes a constraint, not a column"
                else:
                    m = COLUMN.match(raw)
                    if m:
                        target = m.group(1)
                    else:
                        target = last_col
                        why = "trailing block — attached to the last column"
                # A BLOCK THAT OPENS WITH A RULE IS A GROUP HEADER, not a
                # description of the one column that happens to follow it.
                # `── Promoted: token accounting ──` introduces several columns,
                # and filing it under the first would state something false
                # about that column and lose the grouping for the rest.
                is_group = "─" in run[0] or run[0].startswith("──")
                is_hist = any(HISTORY.search(l) for l in run)
                if is_group:
                    dest, note, target = "markdown", "column-group header", None
                elif target is None:
                    dest, note = "markdown", why
                elif is_hist:
                    dest, note = "markdown", "history markers"
                else:
                    dest, note = "catalog", "column semantics"
                out.append({
                    "file": str(path), "schema": schema, "table": table_short,
                    "line": run_at + 1, "length": len(run), "text": run,
                    "column": target, "destination": dest, "note": note,
                })
            run = []
        m = COLUMN.match(raw)
        if m and not CONSTRAINT.match(raw):
            last_col = m.group(1)
    return out


def existing_comment(text: str, table: str, col: str):
    """The span of an existing `comment on column t.col is '…';`, if any."""
    pat = re.compile(rf"comment\s+on\s+column\s+{re.escape(table)}\.{re.escape(col)}\s*\n?\s*is\s*\n?\s*'",
                     re.I)
    m = pat.search(text)
    if not m:
        return None
    i = m.end()
    while i < len(text):
        if text[i] == "'":
            if i + 1 < len(text) and text[i + 1] == "'":
                i += 2
                continue
            j = text.find(";", i)
            return (m.start(), j + 1 if j != -1 else i + 1)
        i += 1
    return None


def sql_quote(s: str) -> str:
    return s.replace("'", "''")


def paragraphs(text_lines: list[str]) -> str:
    """Rejoin a comment block, keeping its paragraph breaks.

    A bare `--` line is a deliberate break between two thoughts. Flattening the
    block with a single join turns four arguments into one wall of text and
    loses the structure the author put there on purpose.
    """
    paras, cur = [], []
    for line in text_lines:
        if line.strip():
            cur.append(line.strip())
        elif cur:
            paras.append(" ".join(cur))
            cur = []
    if cur:
        paras.append(" ".join(cur))
    return "\n\n".join(paras)


def apply_plan(plan: list[dict]) -> int:
    by_file: dict[str, list[dict]] = {}
    for e in plan:
        by_file.setdefault(e["file"], []).append(e)

    md_by_schema: dict[str, list[dict]] = {}
    changed = 0
    for fname, entries in by_file.items():
        path = pathlib.Path(fname)
        text = path.read_text()
        lines = text.split("\n")
        # Remove the blocks bottom-up so earlier line numbers stay valid.
        for e in sorted(entries, key=lambda x: -x["line"]):
            del lines[e["line"] - 1: e["line"] - 1 + e["length"]]
        text = "\n".join(lines)

        for e in entries:
            if e["destination"] != "catalog" or not e["column"]:
                md_by_schema.setdefault(e["schema"], []).append(e)
                continue
            prose = paragraphs(e["text"])
            span = existing_comment(text, e["table"], e["column"])
            if span:
                a, b = span
                stmt = text[a:b]
                body = stmt[stmt.index("'") + 1: stmt.rindex("'")]
                merged = body.rstrip() + "\n\n" + sql_quote(prose)
                text = text[:a] + (f"comment on column {e['table']}.{e['column']}\n"
                                   f"     is '{merged}';") + text[b:]
            else:
                text = text.rstrip("\n") + (
                    f"\ncomment on column {e['table']}.{e['column']}\n"
                    f"     is '{sql_quote(prose)}';\n")
        path.write_text(text)
        changed += 1

    for schema, entries in sorted(md_by_schema.items()):
        md = pathlib.Path(f"docs/database/{schema}.md")
        md.parent.mkdir(parents=True, exist_ok=True)
        head = ""
        if not md.exists():
            head = (f"# `{schema}` — design notes\n\n"
                    f"Why the tables in `database/ddl/table/{schema}/` are shaped the way "
                    f"they are.\n\nWhat a column MEANS lives in `comment on column`, where "
                    f"the database catalog can serve it. This file holds the other half: "
                    f"what was measured, what was tried and failed, and what must not be "
                    f"changed back. It is prose about decisions, which is why it is not in "
                    f"the DDL.\n")
        body = [head] if head else []
        if md.exists():
            body = [md.read_text().rstrip("\n"), ""]
        by_table: dict[str, list[dict]] = {}
        for e in entries:
            by_table.setdefault(e["table"], []).append(e)
        for table, es in sorted(by_table.items()):
            body.append(f"\n## `{schema}.{table}`\n")
            body.append(f"[`database/ddl/table/{schema}/{table}.ddl`]"
                        f"(../../database/ddl/table/{schema}/{table}.ddl)\n")
            for e in es:
                if e["column"]:
                    body.append(f"\n### `{e['column']}`\n")
                body.append("\n" + paragraphs(e["text"]) + "\n")
        md.write_text("\n".join(body).rstrip("\n") + "\n")
        print(f"  wrote {md}")
    return changed


def main() -> int:
    args = sys.argv[1:]
    if "--plan" in args:
        only = None
        if "--only" in args:
            only = args[args.index("--only") + 1]
        files = [pathlib.Path(f) for f in
                 subprocess.run(["git", "ls-files", "database/ddl/table"],
                                capture_output=True, text=True).stdout.split()
                 if f.endswith(".ddl")]
        if only:
            files = [f for f in files if only in str(f)]
        plan = [e for f in files for e in plan_file(f)]
        PLAN.write_text(json.dumps(plan, indent=2))
        cat = sum(1 for e in plan if e["destination"] == "catalog")
        md = len(plan) - cat
        print(f"planned {len(plan)} blocks across {len({e['file'] for e in plan})} files")
        print(f"  -> comment on column : {cat} blocks ({sum(e['length'] for e in plan if e['destination']=='catalog')} lines)")
        print(f"  -> markdown          : {md} blocks ({sum(e['length'] for e in plan if e['destination']=='markdown')} lines)")
        unknown = [e for e in plan if not e["column"]]
        if unknown:
            print(f"  !! {len(unknown)} blocks have no target column:")
            for e in unknown[:10]:
                print(f"       {e['file']}:{e['line']} — {e['note']}")
        print(f"\nwrote {PLAN} — read it before --apply")
        return 0
    if "--apply" in args:
        if not PLAN.is_file():
            sys.exit("no plan at /tmp/ddl-plan.json — run --plan first")
        plan = json.loads(PLAN.read_text())
        n = apply_plan(plan)
        print(f"rewrote {n} DDL files")
        return 0
    print(__doc__)
    return 1


if __name__ == "__main__":
    sys.exit(main())
