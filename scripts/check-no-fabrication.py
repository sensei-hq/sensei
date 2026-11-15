#!/usr/bin/env python3
"""Refuse new failure-masks: a failed read must surface as an error, never a plausible value (D83).

The 2026-11-03 audit (`coord/audits/fabrication-2026-11-03.md`) found 478 places
where a failed read, lookup, parse or RPC turns into a value a caller cannot tell
from a real result: an empty list, a zero, a default config, "daemon · running".
This gate is the ratchet that stops the count from growing while those are fixed.

WHAT IT MATCHES. Narrow failure-mask SHAPES per layer (§6.1 of the audit), not
every `unwrap_or`: most of those are plain `Option`s and correct. Comments are
stripped and string contents blanked before matching, so neither a commented-out
line nor a string that happens to spell a pattern is counted. Rust test code is
exempt by the shared definition in `scripts/_rust_scan.py`; TS spec/test files,
harnesses and generated code are skipped by name.

  R1 Rust  a fallible I/O result (.await, pg., fs::*, .json(), .send(), rusqlite, …)
           ending in .unwrap_or_default() / .unwrap_or( / .unwrap_or_else(|_| / .ok()
           — not when a `?` already propagated the Result, nor a discarded `x.ok();`
  R2 Rust  .ok()? / .ok()?? on an I/O statement
  R3 Rust  an Err(_) / Err(e) arm whose BODY (after any log macro) is a literal
           default — Vec::new(), None, 0, false, "", T::default(), json!([]), a tuple
           of those, Ok(<one of those>) — or a bare return / continue
  R4 Rust  filter_map(|r| r.ok()), filter_map(Result::ok), read_dir/lines .flatten()
  R5 Rust  serde_json/serde_yaml/toml/json5 parse ending in a default
  R6 Rust  let Ok(..) = <I/O or parse> else { <the same literal bodies as R3> }
  T1 TS    .catch(() => [] | null | 0 | {} | '' | false | undefined)
  T2 TS    a catch block that returns a literal / fixture with no error recorded
  T3 TS    ?? [] / ?? 0 / || [] / || 0 in a file that fetches
  T4 Svelte  a literal live status in markup ("daemon · running", ">Connected<")
  T5 TS    dōjō `const { data } = await` without `error`: ABSOLUTE ZERO
  S1 SQL   coalesce(x, 0 | '{}' | '[]' | false | now() | '<word>' | gen_random_uuid()), or a
           last argument built on now(), in view/function/procedure
  S2 SQL   exception when others

THE RATCHET. `scripts/fabrication-baseline.json` holds a count per (rule, file).
The gate fails when any cell rises, and a pair absent from the baseline has a
baseline of 0. Per file, not a total, so a fix in one file cannot pay for a new
mask in another. `--update` rewrites the baseline DOWNWARD only.

THE ALLOWLIST. `scripts/fabrication-allow.tsv` lists reviewed hits that are not
masks (GENUINE-EMPTY, PARSE-DEFAULT, NOT-A-DEFAULT), matched by (file, rule,
whitespace-collapsed line text) — never a line number. An allowlisted hit is not
counted. A row that matches nothing FAILS the check, so the list cannot rot. A
row may never match a known failure-mask (`scripts/fabrication-known-masks.tsv`,
generated from the audit's Appendix A): a mask is fixed, not allowlisted.

Usage:
  check-no-fabrication.py            the gate (make check-no-fabrication)
  check-no-fabrication.py --update   lower the baseline to today's counts; refuses a rise
  check-no-fabrication.py --init     write the first baseline (refuses if one exists)
  check-no-fabrication.py --list     print every counted hit as rule<TAB>file:line<TAB>text
                         [--all]   … and the allowlisted ones too
  check-no-fabrication.py --test     self-test over scripts/fixtures/no-fabrication/
"""

from __future__ import annotations

import json
import os
import pathlib
import re
import sys
from collections import defaultdict

sys.dont_write_bytecode = True  # no __pycache__ beside the scripts
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from _rust_scan import TestCode, blank as _blank, lex_rust  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parent.parent
SCRIPTS = ROOT / "scripts"
BASELINE = SCRIPTS / "fabrication-baseline.json"
ALLOW = SCRIPTS / "fabrication-allow.tsv"
KNOWN_MASKS = SCRIPTS / "fabrication-known-masks.tsv"
FIXTURES = SCRIPTS / "fixtures/no-fabrication"

THE_RULE = "a failed read must surface as an error, never a plausible value (D83)"

RUST_ROOTS = ("crates",)
TS_ROOTS = ("app/src", "dojo/src", "packages")
SQL_ROOTS = ("database/ddl",)
PRUNE_DIRS = {"node_modules", ".svelte-kit", "target", "paraglide", "dist", "build", ".turbo"}
TS_EXTS = (".ts", ".js", ".mjs", ".cjs", ".svelte")
TS_SKIP = re.compile(r"(\.spec\.|\.test\.|\.harness\.|/tests?/|/e2e/|/__fixtures__/|/fixtures/)")
SQL_EXTS = (".ddl", ".sql")
S1_DIRS = ("/view/", "/function/", "/procedure/")

RULES = {
    "rust": ("R1", "R2", "R3", "R4", "R5", "R6"),
    "ts": ("T1", "T2", "T3", "T4", "T5"),
    "sql": ("S1", "S2"),
}
LAYER_OF = {r: layer for layer, rules in RULES.items() for r in rules}
ABSOLUTE = {"T5"}  # zero today; any hit fails, no baseline cell
ALLOW_CLASSES = {"GENUINE-EMPTY", "PARSE-DEFAULT", "NOT-A-DEFAULT"}
ALLOW_COLUMNS = ["file", "rule", "anchor", "class", "reason", "reviewed", "date"]


# ── lexing: comments to spaces, string contents blanked ──────────────────────
#
# Each lexer returns (code, mask), both the same length as the source with every
# newline kept, so offsets and line numbers agree across all three. `code` has
# comments replaced by spaces; `mask` additionally blanks string contents
# (quotes kept), so a pattern spelled inside a string is not code.


_REGEX_PREV = set("(,=:[!&|?{};+-*%<>~^") | {""}
_JS_TOKEN = re.compile(r"//|/\*|['\"`{}/]")


def lex_js(src: str) -> tuple[str, str]:
    code, mask = [], []
    n = len(src)
    brace_stack: list[str] = []  # "t" = a template's ${ }, "b" = a plain brace

    def emit(chunk, masked=None):
        code.append(chunk)
        mask.append(chunk if masked is None else masked)

    def prev_sig() -> str:
        for k in range(len(code) - 1, -1, -1):
            t = code[k].rstrip()
            if t:
                return t[-1]
        return ""

    def template_text(j: int) -> int:
        """Emit template-literal text from j up to its closing backtick or the next
        `${`; return the index at which code scanning resumes."""
        start = j
        while j < n:
            ch = src[j]
            if ch == "\\":
                j += 2
                continue
            if ch == "`":
                emit(src[start:j], _blank(src[start:j]))
                emit("`")
                return j + 1
            if src.startswith("${", j):
                emit(src[start:j], _blank(src[start:j]))
                emit("${")
                brace_stack.append("t")
                return j + 2
            j += 1
        emit(src[start:n], _blank(src[start:n]))
        return n

    i = 0
    while i < n:
        m = _JS_TOKEN.search(src, i)
        if not m:
            emit(src[i:])
            break
        s = m.start()
        if s > i:
            emit(src[i:s])
        tok = m.group(0)
        if tok == "//":
            j = src.find("\n", s)
            j = n if j < 0 else j
            emit(_blank(src[s:j]))
        elif tok == "/*":
            j = src.find("*/", s + 2)
            j = n if j < 0 else j + 2
            emit(_blank(src[s:j]))
        elif tok in "'\"":
            j = s + 1
            while j < n and src[j] != tok and src[j] != "\n":
                j += 2 if src[j] == "\\" else 1
            j = min(j + 1, n)
            body = src[s:j]
            inner_end = len(body) - 1 if body.endswith(tok) and len(body) >= 2 else len(body)
            emit(body, tok + _blank(body[1:inner_end]) + body[inner_end:])
        elif tok == "`":
            emit("`")
            j = template_text(s + 1)
        elif tok == "{":
            brace_stack.append("b")
            emit(tok)
            j = s + 1
        elif tok == "}":
            if (brace_stack.pop() if brace_stack else "b") == "t":
                emit("}")
                j = template_text(s + 1)
            else:
                emit(tok)
                j = s + 1
        elif prev_sig() in _REGEX_PREV:
            # A regex literal: to the closing unescaped `/` outside a class, same line.
            j, in_class = s + 1, False
            while j < n and src[j] != "\n":
                if src[j] == "\\":
                    j += 2
                    continue
                if src[j] == "[":
                    in_class = True
                elif src[j] == "]":
                    in_class = False
                elif src[j] == "/" and not in_class:
                    break
                j += 1
            if j < n and src[j] == "/":
                body = src[s:j + 1]
                emit(body, "/" + _blank(body[1:-1]) + "/")
                j += 1
            else:
                emit("/")
                j = s + 1
        else:
            emit("/")
            j = s + 1
        i = j
    return "".join(code), "".join(mask)


def lex_svelte(src: str) -> tuple[str, str]:
    """Script blocks are JS; markup only loses its `<!-- -->` comments."""
    code, mask = [], []
    pos = 0
    for m in re.finditer(r"(<script\b[^>]*>)(.*?)(</script>)", src, re.S):
        markup = _strip_html_comments(src[pos:m.start(2)])
        code.append(markup)
        mask.append(markup)
        c, k = lex_js(m.group(2))
        code.append(c)
        mask.append(k)
        pos = m.end(2)
    markup = _strip_html_comments(src[pos:])
    code.append(markup)
    mask.append(markup)
    return "".join(code), "".join(mask)


def _strip_html_comments(s: str) -> str:
    return re.sub(r"<!--.*?-->", lambda m: _blank(m.group(0)), s, flags=re.S)


_SQL_TOKEN = re.compile(r"--|/\*|['\"]")


def lex_sql(src: str) -> tuple[str, str]:
    """`--` and `/* */` comments go; '...' strings are blanked in the mask.
    Dollar-quoted bodies are code (a plpgsql body is where the shapes live)."""
    code, mask = [], []
    i, n = 0, len(src)
    while i < n:
        m = _SQL_TOKEN.search(src, i)
        if not m:
            code.append(src[i:])
            mask.append(src[i:])
            break
        s = m.start()
        code.append(src[i:s])
        mask.append(src[i:s])
        tok = m.group(0)
        if tok == "--":
            j = src.find("\n", s)
            j = n if j < 0 else j
            code.append(_blank(src[s:j]))
            mask.append(_blank(src[s:j]))
        elif tok == "/*":
            j = src.find("*/", s + 2)
            j = n if j < 0 else j + 2
            code.append(_blank(src[s:j]))
            mask.append(_blank(src[s:j]))
        else:
            j = s + 1
            while j < n:
                if src[j] == tok:
                    if j + 1 < n and src[j + 1] == tok:
                        j += 2
                        continue
                    break
                j += 1
            j = min(j + 1, n)
            body = src[s:j]
            code.append(body)
            mask.append(tok + _blank(body[1:-1]) + body[-1:] if len(body) >= 2 else body)
        i = j
    return "".join(code), "".join(mask)


# ── shared helpers ────────────────────────────────────────────────────────────


def line_of(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def balanced(text: str, open_at: int) -> int:
    """Index just past the bracket matching the one at `open_at`."""
    pairs = {"(": ")", "[": "]", "{": "}"}
    stack = [pairs[text[open_at]]]
    j = open_at + 1
    while j < len(text) and stack:
        ch = text[j]
        if ch in pairs:
            stack.append(pairs[ch])
        elif ch == stack[-1]:
            stack.pop()
        elif ch in ")]}":
            return j + 1  # unbalanced: stop here rather than run to EOF
        j += 1
    return j


def anchor(raw_line: str) -> str:
    return " ".join(raw_line.split())


# ── Rust rules ────────────────────────────────────────────────────────────────

IO = re.compile(
    r"\.await\b|\bpg\.|read_to_string\s*\(|\bfs::\w+\s*\(|File::open|read_dir\s*\(|"
    r"\.output\s*\(\s*\)|\.json\s*(?:::<[^>]*>)?\s*\(|\.text\s*\(\s*\)|\.send\s*\(|"
    r"Connection::open\w*\s*\(|\.prepare\s*\(|\.query_(?:row|map)\s*\("
)
PARSE = re.compile(
    r"\b(?:serde_json|serde_yaml|toml|json5)::from_\w+|\.parse\s*::<\s*(?:serde_json|serde_yaml|toml)::"
)
DEFAULT_END = re.compile(
    r"\.unwrap_or_default\s*\(\s*\)|\.unwrap_or\s*\(|\.unwrap_or_else\s*\(\s*\|\s*_\s*\||\.ok\s*\(\s*\)(?!\s*\?)"
)
OK_Q = re.compile(r"\.ok\s*\(\s*\)\s*\?")
ERR_ARM = re.compile(r"\bErr\s*\(\s*(?:_|[a-z_]\w*)\s*\)\s*=>")
LET_OK = re.compile(r"\blet\s+Ok\s*\(")
FILTER_OK = re.compile(r"filter_map\s*\(\s*(?:Result::ok\s*\)|\|[^|]*\|[^\n]*?\.ok\s*\(\s*\)\s*\))")
FLATTEN = re.compile(r"\.flatten\s*\(\s*\)")
FLATTEN_SRC = re.compile(r"read_dir\s*\(|\.lines\s*\(\s*\)")
LOG_MACRO = re.compile(r"\s*(?:(?:tracing|log)::)?(?:warn|error|info|debug|trace|eprintln|println)!\s*\(")
_LIT_ATOM = (
    r"(?:(?:std::)?vec::Vec::new\(\)|Vec::new\(\)|vec!\[\s*\]|None|0|0\.0|0_?(?:u|i)(?:8|16|32|64|128|size)|"
    r"0(?:\.0)?_?f(?:32|64)|false|String::new\(\)|\"\s*\"(?:\.to_string\(\)|\.into\(\)|\.to_owned\(\))?|"
    r"Default::default\(\)|[A-Z]\w*::default\(\)|json!\(\s*\[\s*\]\s*\)|json!\(\s*\{\s*\}\s*\)|"
    r"(?:std::collections::)?(?:Hash|BTree)(?:Map|Set)::new\(\))"
)
_LIT_VALUE = r"(?:" + _LIT_ATOM + r"|\(\s*" + _LIT_ATOM + r"(?:\s*,\s*" + _LIT_ATOM + r")+\s*\))"
LIT_DEFAULT = re.compile(
    r"(?:(?:return\s+)?(?:Ok\s*\(\s*)?" + _LIT_VALUE + r"(?:\s*\))?|return|continue)\s*;?"
)
STMT_END = (";", "{", "}", ",")


def _statement_before(lines: list[str], idx: int, col: int) -> str:
    """The statement text that ends at (idx, col): walks back over a method chain."""
    parts = [lines[idx][:col]]
    k = idx - 1
    while k >= 0 and idx - k <= 12:
        prev = lines[k].rstrip()
        if not prev.strip():
            k -= 1
            continue
        if prev.endswith(STMT_END):
            break
        parts.append(prev)
        k -= 1
    return "\n".join(reversed(parts))


def _is_literal_body(body: str) -> bool:
    body = body.strip()
    while True:
        m = LOG_MACRO.match(body)
        if not m:
            break
        end = balanced(body, m.end() - 1)
        body = body[end:].lstrip().lstrip(";").strip()
    return bool(LIT_DEFAULT.fullmatch(" ".join(body.split())))


def _arm_body(mask: str, after: int) -> str:
    j = after
    while j < len(mask) and mask[j] in " \t\r\n":
        j += 1
    if j < len(mask) and mask[j] == "{":
        end = balanced(mask, j)
        return mask[j + 1:end - 1]
    depth, k = 0, j
    while k < len(mask):
        ch = mask[k]
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            if depth == 0:
                break
            depth -= 1
        elif ch == "," and depth == 0:
            break
        elif ch == "\n" and depth == 0:
            break
        k += 1
    return mask[j:k]


def rust_hits(mask: str) -> dict[int, str]:
    """{line number: rule} — at most one rule per line, the most specific."""
    found: dict[int, set[str]] = defaultdict(set)
    lines = mask.split("\n")

    for m in ERR_ARM.finditer(mask):
        if _is_literal_body(_arm_body(mask, m.end())):
            found[line_of(mask, m.start())].add("R3")

    for m in LET_OK.finditer(mask):
        rest = mask[m.end():m.end() + 2000]
        e = re.search(r"\belse\s*\{", rest)
        semi = rest.find(";")
        if not e or (0 <= semi < e.start()):
            continue
        head = rest[:e.start()]
        if not (IO.search(head) or PARSE.search(head)):
            continue
        open_at = m.end() + e.end() - 1
        body = mask[open_at + 1:balanced(mask, open_at) - 1]
        if _is_literal_body(body):
            found[line_of(mask, m.start())].add("R6")

    for idx, line in enumerate(lines):
        n = idx + 1
        if FILTER_OK.search(line):
            found[n].add("R4")
        fm = FLATTEN.search(line)
        if fm and FLATTEN_SRC.search(_statement_before(lines, idx, fm.start())):
            found[n].add("R4")
        for m in DEFAULT_END.finditer(line):
            stmt = _statement_before(lines, idx, m.start())
            if m.group(0).startswith(".ok") and _discarded(stmt, line[m.end():]):
                continue
            pm = list(PARSE.finditer(stmt))
            if pm and "?" not in stmt[pm[-1].end():]:
                found[n].add("R5")
                continue
            io = list(IO.finditer(stmt))
            if io and "?" not in stmt[io[-1].end():]:
                found[n].add("R1")
        for m in OK_Q.finditer(line):
            stmt = _statement_before(lines, idx, m.start())
            if PARSE.search(stmt):
                found[n].add("R5")
            elif IO.search(stmt):
                found[n].add("R2")

    order = ("R3", "R6", "R4", "R5", "R2", "R1")
    return {n: next(r for r in order if r in rules) for n, rules in found.items()}


def _discarded(stmt: str, after: str) -> bool:
    """`tx.send(x).await.ok();` drops the error and substitutes nothing."""
    if not after.lstrip().startswith(";"):
        return False
    head = " ".join(stmt.split())
    return not re.search(r"^(?:let|return)\b|[^=!<>]=[^=>]", head)


# ── TS / Svelte rules ─────────────────────────────────────────────────────────

T_LIT = r"(?:\[\s*\]|null|undefined|0|false|\{\s*\}|\(\s*\{\s*\}\s*\)|''|\"\"|``)"
T1 = re.compile(
    r"\.catch\s*\(\s*(?:async\s*)?(?:\(\s*[\w$]*\s*(?::\s*[\w$]+)?\s*\)|[\w$]+)\s*=>\s*" + T_LIT + r"\s*\)"
)
T2_OPEN = re.compile(r"(?<![.\w$])catch\s*(?:\(\s*[^)]*\))?\s*\{")
T2_RETURN = re.compile(r"\breturn\s+(?:" + T_LIT + r"|[\w$]*(?:Fixture|Mock)[\w$]*)\s*;?\s*$")
T2_ASSIGN = re.compile(r"[\w$.\]]\s*=\s*(?:\[\s*\]|0)\s*;?\s*$")
T2_RECORDS = re.compile(r"throw|error|notice|fault|status", re.I)
T3 = re.compile(r"(?:\?\?|\|\|)\s*(?:\[\s*\]|0(?![\w.]))")
FETCHES = re.compile(r"\bfetch\s*\(|\binvoke\s*[<(]|\bapi\.\w+\s*[<(]|\.rpc\s*\(|\.from\s*\(\s*['\"`]")
T4 = re.compile(
    r"(?i:(?:daemon|status|sync)\s*·\s*(?:running|online|connected|healthy))|>\s*(?:Running|Online|Connected|Free)\s*<"
)
T5 = re.compile(r"\bconst\s*\{([^}]*)\}\s*=\s*await\b")


def ts_hits(mask: str, rel: str, is_svelte: bool) -> dict[int, str]:
    found: dict[int, set[str]] = defaultdict(set)
    for m in T1.finditer(mask):
        found[line_of(mask, m.start())].add("T1")
    for m in T2_OPEN.finditer(mask):
        open_at = m.end() - 1
        body = mask[open_at + 1:balanced(mask, open_at) - 1]
        if T2_RECORDS.search(body):
            continue
        first = [ln.strip() for ln in body.split("\n") if ln.strip()][:4]
        if any(T2_RETURN.search(ln) or T2_ASSIGN.search(ln) for ln in first):
            found[line_of(mask, m.start())].add("T2")
    if FETCHES.search(mask):
        for idx, line in enumerate(mask.split("\n")):
            if T3.search(line):
                found[idx + 1].add("T3")
    if is_svelte:
        for idx, line in enumerate(mask.split("\n")):
            if T4.search(line):
                found[idx + 1].add("T4")
    if rel.startswith("dojo/"):
        for m in T5.finditer(mask):
            names = m.group(1)
            if re.search(r"\bdata\b", names) and not re.search(r"\berror\b", names):
                found[line_of(mask, m.start())].add("T5")
    order = ("T5", "T1", "T2", "T4", "T3")
    return {n: next(r for r in order if r in rules) for n, rules in found.items()}


# ── SQL rules ─────────────────────────────────────────────────────────────────

COALESCE = re.compile(r"\bcoalesce\s*\(", re.I)
S1_LAST = re.compile(r"0|0\.0|'\{\}'|'\[\]'|false|now\(\)|current_timestamp|''|'\w+'|gen_random_uuid\(\)")
S1_CLOCK = re.compile(r"\bnow\(\)|\bcurrent_timestamp\b")  # "when it happened" invented as "now"
SQL_CAST = re.compile(r"\s*::\s*[a-z_][\w ]*(?:\([^)]*\))?(?:\[\])?", re.I)
S2 = re.compile(r"\bexception\s+when\s+others\b", re.I)


def sql_hits(code: str, mask: str, rel: str) -> dict[int, str]:
    found: dict[int, str] = {}
    if any(d in "/" + rel for d in S1_DIRS):
        for m in COALESCE.finditer(mask):
            open_at = m.end() - 1
            close = balanced(mask, open_at)
            # Split on top-level commas using the mask; read the text from code.
            depth, last = 0, open_at + 1
            for j in range(open_at + 1, close - 1):
                ch = mask[j]
                if ch in "([":
                    depth += 1
                elif ch in ")]":
                    depth -= 1
                elif ch == "," and depth == 0:
                    last = j + 1
            if last == open_at + 1:
                continue  # one argument: nothing substituted
            arg = SQL_CAST.sub("", code[last:close - 1]).strip().lower()
            if S1_LAST.fullmatch(" ".join(arg.split())) or S1_CLOCK.search(arg):
                found[line_of(mask, m.start())] = "S1"
    for m in S2.finditer(mask):
        found.setdefault(line_of(mask, m.start()), "S2")
    return found


# ── the scan ──────────────────────────────────────────────────────────────────


class Hit:
    __slots__ = ("rule", "file", "line", "text")

    def __init__(self, rule, file, line, text):
        self.rule, self.file, self.line, self.text = rule, file, line, text

    def key(self):
        return (self.file, self.rule, self.text)


def _walk(root: pathlib.Path, sub: str, exts) -> list[pathlib.Path]:
    base = root / sub
    out = []
    if not base.exists():
        return out
    for dirpath, dirnames, filenames in os.walk(base):
        dirnames[:] = sorted(d for d in dirnames if d not in PRUNE_DIRS)
        for f in sorted(filenames):
            if f.endswith(exts):
                out.append(pathlib.Path(dirpath) / f)
    return out


class Scan:
    """Every rule hit under `root`, split into counted and test-exempt."""

    def __init__(self, root: pathlib.Path):
        self.root = root
        self.hits: list[Hit] = []        # production hits (allowlist not yet applied)
        self.test_hits: list[Hit] = []   # hits inside test code, never counted
        self.files = {"rust": 0, "ts": 0, "sql": 0}
        self._run()

    def _rel(self, p: pathlib.Path) -> str:
        return str(p.relative_to(self.root)).replace("\\", "/")

    def _add(self, found: dict[int, str], rel: str, raw: list[str], is_test) -> None:
        for n in sorted(found):
            h = Hit(found[n], rel, n, anchor(raw[n - 1] if n - 1 < len(raw) else ""))
            (self.test_hits if is_test(n) else self.hits).append(h)

    def _run(self) -> None:
        rust_files = [p for r in RUST_ROOTS for p in _walk(self.root, r, (".rs",))]
        tests = TestCode(self.root, rust_files)
        for p in rust_files:
            src = p.read_text(errors="replace")
            self.files["rust"] += 1
            _, mask = lex_rust(src)
            self._add(rust_hits(mask), self._rel(p), src.split("\n"),
                      lambda n, p=p, mask=mask: tests.is_test_line(p, n, mask))
        for p in (p for r in TS_ROOTS for p in _walk(self.root, r, TS_EXTS)):
            rel = self._rel(p)
            src = p.read_text(errors="replace")
            self.files["ts"] += 1
            svelte = rel.endswith(".svelte")
            _, mask = lex_svelte(src) if svelte else lex_js(src)
            skip = bool(TS_SKIP.search("/" + rel))
            self._add(ts_hits(mask, rel, svelte), rel, src.split("\n"), lambda n, s=skip: s)
        for p in (p for r in SQL_ROOTS for p in _walk(self.root, r, SQL_EXTS)):
            rel = self._rel(p)
            src = p.read_text(errors="replace")
            self.files["sql"] += 1
            code, mask = lex_sql(src)
            self._add(sql_hits(code, mask, rel), rel, src.split("\n"), lambda n: False)


# ── allowlist, known masks, baseline ──────────────────────────────────────────


def read_tsv(path: pathlib.Path) -> list[dict]:
    if not path.exists():
        return []
    lines = path.read_text().splitlines()
    if not lines:
        return []
    head = lines[0].split("\t")
    rows = []
    for ln in lines[1:]:
        if not ln.strip() or ln.startswith("#"):
            continue
        cells = ln.split("\t")
        rows.append(dict(zip(head, cells + [""] * (len(head) - len(cells)))))
    return rows


def allow_problems(allow: list[dict], masks: list[dict]) -> list[str]:
    """Rows that can never be valid, whatever the tree holds."""
    out = []
    mask_keys = {(m["file"], m["anchor"]) for m in masks if m.get("anchor")}
    for r in allow:
        where = f"{r['file']} {r['rule']} «{r['anchor']}»"
        if r.get("class") not in ALLOW_CLASSES:
            out.append(f"class {r.get('class')!r} cannot be allowlisted (only {', '.join(sorted(ALLOW_CLASSES))}): {where}")
        if r.get("rule") not in LAYER_OF:
            out.append(f"unknown rule {r.get('rule')!r}: {where}")
        if r.get("rule") in ABSOLUTE:
            out.append(f"{r['rule']} is an absolute zero and has no allowlist: {where}")
        if not r.get("reason"):
            out.append(f"no reason given: {where}")
        if (r["file"], r["anchor"]) in mask_keys:
            out.append(f"matches a known FAILURE-MASK from the audit's Appendix A — fix it, do not allowlist it: {where}")
    return out


class Result:
    def __init__(self, scan: Scan, allow: list[dict]):
        allowed_keys = {(r["file"], r["rule"], r["anchor"]) for r in allow}
        self.counted = [h for h in scan.hits if h.key() not in allowed_keys]
        self.allowed = [h for h in scan.hits if h.key() in allowed_keys]
        live = {h.key() for h in scan.hits}
        self.stale = [r for r in allow if (r["file"], r["rule"], r["anchor"]) not in live]
        self.counts: dict[str, dict[str, int]] = defaultdict(lambda: defaultdict(int))
        for h in self.counted:
            self.counts[h.rule][h.file] += 1


def ratchet_cells(counts) -> dict[str, dict[str, int]]:
    return {r: dict(sorted(files.items())) for r, files in sorted(counts.items()) if r not in ABSOLUTE and files}


def compare(baseline: dict, counts) -> tuple[list, list]:
    """(rises, drops): each (rule, file, old, new)."""
    rises, drops = [], []
    cells = {(r, f) for r, fs in counts.items() for f in fs} | {(r, f) for r, fs in baseline.items() for f in fs}
    for rule, f in sorted(cells):
        if rule in ABSOLUTE:
            continue
        old = baseline.get(rule, {}).get(f, 0)
        new = counts.get(rule, {}).get(f, 0)
        if new > old:
            rises.append((rule, f, old, new))
        elif new < old:
            drops.append((rule, f, old, new))
    return rises, drops


def empty_layers(baseline: dict, scan: Scan, result: Result) -> list[str]:
    """A layer whose baseline is non-zero but which produced no hit read nothing."""
    out = []
    for layer, rules in RULES.items():
        expected = sum(sum(baseline.get(r, {}).values()) for r in rules)
        seen = sum(1 for h in scan.hits if LAYER_OF[h.rule] == layer)
        if expected and (not seen or not scan.files[layer]):
            out.append(f"{layer}: baseline holds {expected} but the scan found none "
                       f"({scan.files[layer]} files read) — the scan read nothing, which is not a pass")
    return out


def update_baseline(baseline: dict, counts) -> tuple[dict | None, list, list]:
    """(new baseline, rises, drops). The new baseline is None when anything rose."""
    rises, drops = compare(baseline, counts)
    if rises:
        return None, rises, drops
    return ratchet_cells(counts), rises, drops


# ── reporting ─────────────────────────────────────────────────────────────────


def _layer_table(baseline: dict, result: Result) -> list[str]:
    out = []
    allowed = defaultdict(int)
    for h in result.allowed:
        allowed[h.rule] += 1
    for layer, rules in RULES.items():
        parts = []
        for r in rules:
            cur = sum(result.counts.get(r, {}).values())
            if r in ABSOLUTE:
                parts.append(f"{r} {cur}/0 (absolute)")
            else:
                parts.append(f"{r} {cur}/{sum(baseline.get(r, {}).values())}" + (f" +{allowed[r]} allowed" if allowed[r] else ""))
        out.append(f"  {layer:5} " + "  ".join(parts))
    return out


def gate(root: pathlib.Path, baseline_path: pathlib.Path, allow_path: pathlib.Path,
         masks_path: pathlib.Path, out=print) -> int:
    scan = Scan(root)
    allow = read_tsv(allow_path)
    masks = read_tsv(masks_path)
    result = Result(scan, allow)
    baseline = json.loads(baseline_path.read_text()) if baseline_path.exists() else None
    failures = 0

    if baseline is None:
        out(f"check-no-fabrication: no baseline at {baseline_path.relative_to(ROOT)} — run with --init once.")
        return 1

    problems = allow_problems(allow, masks)
    rises, drops = compare(baseline, result.counts)
    absolute = [h for h in result.counted if h.rule in ABSOLUTE]
    empties = empty_layers(baseline, scan, result)

    for p in problems:
        out(f"check-no-fabrication: allowlist row rejected: {p}")
    failures += len(problems)
    if result.stale:
        out(f"check-no-fabrication: {len(result.stale)} stale allowlist row(s) — the line no longer exists; "
            f"delete the row from {allow_path.relative_to(ROOT) if allow_path.is_relative_to(ROOT) else allow_path}:")
        for r in result.stale:
            out(f"  {r['file']}  {r['rule']}  «{r['anchor']}»")
        failures += len(result.stale)
    for e in empties:
        out(f"check-no-fabrication: {e}")
    failures += len(empties)

    by_cell = defaultdict(list)
    for h in result.counted:
        by_cell[(h.rule, h.file)].append(h)
    if rises:
        out(f"check-no-fabrication: {len(rises)} (rule, file) count(s) rose — a new failure-mask shape:\n")
        for rule, f, old, new in rises:
            label = "new file for this rule" if old == 0 else "rose"
            out(f"  {rule}  {f}  {old} → {new}  ({label})")
            for h in by_cell[(rule, f)]:
                out(f"      {f}:{h.line}  {h.text[:140]}")
        failures += len(rises)
    if absolute:
        out(f"check-no-fabrication: {len(absolute)} T5 hit(s) — a dōjō read that drops its `error` (absolute zero):")
        for h in absolute:
            out(f"      {h.file}:{h.line}  {h.text[:140]}")
        failures += len(absolute)

    if failures:
        out("\n  " + THE_RULE + ".")
        out("  Return the error (Err / 4xx-5xx / an error state the UI renders). If the hit is a\n"
            "  reviewed genuine-empty or parse-default, add a row to scripts/fabrication-allow.tsv\n"
            "  (file, rule, the exact line text, class, reason) instead of raising the baseline.")
        out("\n" + "\n".join(_layer_table(baseline, result)))
        return 1

    total = len(result.counted)
    out(f"check-no-fabrication: {total} known failure-mask shape(s), none new "
        f"({len(result.allowed)} allowlisted, {len(scan.test_hits)} in test code).")
    out("\n".join(_layer_table(baseline, result)))
    if drops:
        out(f"  {len(drops)} cell(s) fell below the baseline — lock them in with "
            f"`scripts/check-no-fabrication.py --update`.")
    return 0


def run_update(root, baseline_path, allow_path, init=False, out=print) -> int:
    scan = Scan(root)
    result = Result(scan, read_tsv(allow_path))
    if init:
        if baseline_path.exists():
            out(f"check-no-fabrication --init: {baseline_path.name} already exists; use --update.")
            return 1
        baseline_path.write_text(json.dumps(ratchet_cells(result.counts), indent=1, sort_keys=True) + "\n")
        out(f"check-no-fabrication --init: wrote {sum(map(len, ratchet_cells(result.counts).values()))} cells, "
            f"{sum(sum(v.values()) for v in ratchet_cells(result.counts).values())} hits.")
        return 0
    baseline = json.loads(baseline_path.read_text())
    new, rises, drops = update_baseline(baseline, result.counts)
    if new is None:
        out("check-no-fabrication --update: refused — the baseline only goes down. These rose:")
        for rule, f, old, now in rises:
            out(f"  {rule}  {f}  {old} → {now}")
        return 1
    baseline_path.write_text(json.dumps(new, indent=1, sort_keys=True) + "\n")
    if not drops:
        out("check-no-fabrication --update: nothing fell; baseline unchanged.")
    for rule, f, old, now in drops:
        out(f"  fell: {rule}  {f}  {old} → {now}")
    return 0


# ── self-test ─────────────────────────────────────────────────────────────────

EXPECT = re.compile(r"expect:\s*([A-Z]\d)(\s+allowed)?")


def self_test() -> int:
    fails: list[str] = []

    def check(cond, msg):
        if not cond:
            fails.append(msg)

    scan = Scan(FIXTURES)
    want, want_allowed = set(), set()
    for layer_root in RUST_ROOTS + TS_ROOTS + SQL_ROOTS:
        for p in _walk(FIXTURES, layer_root, (".rs",) + TS_EXTS + SQL_EXTS):
            rel = str(p.relative_to(FIXTURES))
            for n, line in enumerate(p.read_text().split("\n"), 1):
                m = EXPECT.search(line)
                if m:
                    (want_allowed if m.group(2) else want).add((rel, n, m.group(1)))
    got = {(h.file, h.line, h.rule) for h in scan.hits}
    for miss in sorted((want | want_allowed) - got):
        check(False, f"must-catch not caught: {miss[0]}:{miss[1]} {miss[2]}")
    for extra in sorted(got - want - want_allowed):
        check(False, f"must-not-catch was caught: {extra[0]}:{extra[1]} {extra[2]}")
    for rule in sorted(LAYER_OF):
        check(any(r == rule for _, _, r in want), f"rule {rule} has no must-catch fixture")

    # Test code is exempt: three whole test files, a #[cfg(test)] fn and module, a spec file.
    test_files = {h.file for h in scan.test_hits}
    for f in ("crates/demo/src/quality/mod.rs", "crates/demo/src/store_tests.rs",
              "crates/demo/tests/integration.rs", "crates/demo/src/lib.rs", "app/src/lib/load.spec.ts"):
        check(f in test_files, f"no test-exempt hit recorded for {f} (the exemption is untested)")

    # Allowlist: the allowed fixture is not counted; nothing is stale.
    allow = read_tsv(FIXTURES / "allow.tsv")
    res = Result(scan, allow)
    check({(h.file, h.line, h.rule) for h in res.allowed} == want_allowed,
          f"allowlisted hits {sorted((h.file, h.line) for h in res.allowed)} != {sorted(want_allowed)}")
    check(not res.stale, f"fixture allowlist reported stale rows: {res.stale}")
    check(not any(h.key() in {(r['file'], r['rule'], r['anchor']) for r in allow} for h in res.counted),
          "an allowlisted hit was still counted")

    # A stale row fails.
    stale = Result(scan, read_tsv(FIXTURES / "allow-stale.tsv"))
    check(len(stale.stale) == 1, f"stale allow row not detected ({len(stale.stale)} stale)")

    # An allow row that matches a known failure-mask is rejected; so is a FAILURE-MASK class.
    masks = read_tsv(FIXTURES / "known-masks.tsv")
    check(any("known FAILURE-MASK" in p for p in allow_problems(allow, masks)),
          "allow row matching a known mask was not rejected")
    bad_class = [dict(allow[0], **{"class": "FAILURE-MASK"})]
    check(any("cannot be allowlisted" in p for p in allow_problems(bad_class, [])),
          "a FAILURE-MASK class row was accepted")
    check(not allow_problems(allow, []), f"valid allow row rejected: {allow_problems(allow, [])}")

    # Conservation: every raw hit lands in exactly one of counted / allowed / test.
    raw = 0
    for layer_root in RUST_ROOTS + TS_ROOTS + SQL_ROOTS:
        for p in _walk(FIXTURES, layer_root, (".rs",) + TS_EXTS + SQL_EXTS):
            rel = str(p.relative_to(FIXTURES))
            src = p.read_text()
            if p.suffix == ".rs":
                raw += len(rust_hits(lex_rust(src)[1]))
            elif p.suffix in SQL_EXTS:
                raw += len(sql_hits(*lex_sql(src), rel))
            else:
                sv = rel.endswith(".svelte")
                raw += len(ts_hits((lex_svelte(src) if sv else lex_js(src))[1], rel, sv))
    parts = len(res.counted) + len(res.allowed) + len(scan.test_hits)
    check(raw == parts, f"conservation: raw {raw} != counted {len(res.counted)} + allowed "
                        f"{len(res.allowed)} + test {len(scan.test_hits)}")
    check(len(scan.test_hits) == 8, f"expected 8 test-exempt hits, got {len(scan.test_hits)}")

    # The ratchet.
    base = {"R1": {"a.rs": 2}, "S1": {"v.ddl": 1}}
    rises, drops = compare(base, {"R1": {"a.rs": 3}, "S1": {"v.ddl": 1}})
    check(rises == [("R1", "a.rs", 2, 3)] and not drops, f"a rise was not caught: {rises} {drops}")
    rises, _ = compare(base, {"R1": {"a.rs": 2, "b.rs": 1}, "S1": {"v.ddl": 1}})
    check(rises == [("R1", "b.rs", 0, 1)], f"a new (rule, file) pair was not caught: {rises}")
    rises, drops = compare(base, {"R1": {"a.rs": 1}})
    check(not rises and len(drops) == 2, f"drops misread: {rises} {drops}")
    new, rises, _ = update_baseline(base, {"R1": {"a.rs": 3}})
    check(new is None and rises, "--update raised a cell")
    new, _, drops = update_baseline(base, {"R1": {"a.rs": 1}})
    check(new == {"R1": {"a.rs": 1}} and len(drops) == 2, f"--update did not lower: {new} {drops}")
    rises, _ = compare({}, {"T5": {"dojo/x.ts": 1}})
    check(not rises, "T5 entered the ratchet; it is an absolute zero")

    # A layer with a non-zero baseline and no hits is an error, not a pass.
    empty = Scan.__new__(Scan)
    empty.root, empty.hits, empty.test_hits, empty.files = FIXTURES, [], [], {"rust": 0, "ts": 0, "sql": 0}
    check(len(empty_layers(base, empty, Result(empty, []))) == 2, "an empty scan passed")

    # The whole gate over the fixtures, end to end: the T5 fixture must fail it.
    lines: list[str] = []
    import tempfile
    with tempfile.TemporaryDirectory() as td:
        bp = pathlib.Path(td) / "baseline.json"
        bp.write_text(json.dumps(ratchet_cells(res.counts)))
        rc = gate(FIXTURES, bp, FIXTURES / "allow.tsv", pathlib.Path(td) / "none.tsv", out=lines.append)
        check(rc == 1 and any("T5" in ln for ln in lines), "the absolute T5 fixture did not fail the gate")
        check(any(THE_RULE in ln for ln in lines), "failure output does not state the rule")

    if fails:
        print("check-no-fabrication --test: FAILED")
        for f in fails:
            print(f"  - {f}")
        return 1
    print(f"check-no-fabrication --test: ok — {len(want)} must-catch, {len(want_allowed)} allowlisted, "
          f"{len(scan.test_hits)} test-exempt, conservation {raw} = {parts}.")
    return 0


def main(argv: list[str]) -> int:
    if "--test" in argv:
        return self_test()
    if "--list" in argv:
        scan = Scan(ROOT)
        res = Result(scan, read_tsv(ALLOW))
        shown = res.counted + (res.allowed if "--all" in argv else [])
        for h in sorted(shown, key=lambda h: (h.file, h.line)):
            print(f"{h.rule}\t{h.file}:{h.line}\t{h.text}")
        return 0
    if "--update" in argv or "--init" in argv:
        return run_update(ROOT, BASELINE, ALLOW, init="--init" in argv)
    return gate(ROOT, BASELINE, ALLOW, KNOWN_MASKS)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
