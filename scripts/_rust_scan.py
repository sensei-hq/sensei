"""Rust source scanning shared by the static gates that read crates/.

`check-sql-in-layer.py` (#227) and `check-no-fabrication.py` (D83) both exempt
test code from a production-only rule, and both must not be fooled by a comment
or a string. One lexer and one definition of "test code" serve both, so a test
module one gate learns to skip is skipped by the other.

WHAT IS TEST CODE.
- Any item under a `#[cfg(test)]` attribute in a production file: an inline
  `mod tests { … }`, and equally a lone `#[cfg(test)] fn helper() { … }`. The
  exemption covers exactly that item's lines (its braces, matched on the lexed
  source so a brace in a string or comment does not count). It is by ITEM, not by
  filename or line position, so moving a statement above a marker to dodge a gate
  does not work — and production code after a test-only helper is still scanned.
- A whole file that is a test module but carries no marker of its own, because
  its `#[cfg(test)] mod x;` declaration lives in the parent. Recognised two ways:
  by name (`tests.rs`, `*_tests.rs`, `test_support.rs`, `test_db.rs`, anything
  under a `tests/` directory) and by declaration (any `#[cfg(test)] mod x;`, such
  as `indexer/quality`, makes `x.rs` and everything under `x/` test code).
"""

from __future__ import annotations

import bisect
import pathlib
import re

# Whole-file test modules recognised by name.
TEST_FILE_SUFFIXES = ("/tests.rs", "_tests.rs", "/test_support.rs", "/test_db.rs")
TEST_DIR_PARTS = ("/tests/",)

_CFG_TEST = re.compile(r"^[ \t]*#\[cfg\(test\)\]", re.M)
_MOD_DECL = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;")
_SKIPPABLE = re.compile(r"^\s*(?:$|//|#\[)")


_NOT_NEWLINE = re.compile(r"[^\n]")


def blank(s: str) -> str:
    """`s` with every character but a newline turned into a space."""
    return _NOT_NEWLINE.sub(" ", s) if "\n" in s else " " * len(s)


_blank = blank


_RUST_TOKEN = re.compile(r"""//|/\*|b?r#*"|b?"|'""")


def lex_rust(src: str) -> tuple[str, str]:
    """(code, mask), each the length of `src` with every newline kept.

    `code` has comments replaced by spaces; `mask` also blanks string and char
    contents (quotes kept), so a pattern spelled inside a literal is not code.
    """
    code, mask = [], []
    i, n = 0, len(src)
    while i < n:
        m = _RUST_TOKEN.search(src, i)
        if not m:
            code.append(src[i:])
            mask.append(src[i:])
            break
        s = m.start()
        tok = m.group(0)
        # `r"`/`b"` only start a literal when not the tail of an identifier.
        if tok[0] in "br" and s > 0 and (src[s - 1].isalnum() or src[s - 1] == "_"):
            code.append(src[i:s + 1])
            mask.append(src[i:s + 1])
            i = s + 1
            continue
        code.append(src[i:s])
        mask.append(src[i:s])
        if tok == "//":
            j = src.find("\n", s)
            j = n if j < 0 else j
            code.append(_blank(src[s:j]))
            mask.append(_blank(src[s:j]))
        elif tok == "/*":
            depth, j = 1, s + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            code.append(_blank(src[s:j]))
            mask.append(_blank(src[s:j]))
        elif tok.endswith('"') and "r" in tok:
            close = '"' + "#" * tok.count("#")
            j = src.find(close, s + len(tok))
            j = n if j < 0 else j + len(close)
            body = src[s:j]
            code.append(body)
            mask.append(tok + _blank(body[len(tok):len(body) - len(close)]) + body[len(body) - len(close):])
        elif tok.endswith('"'):
            j = s + len(tok)
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            j = min(j + 1, n)
            body = src[s:j]
            code.append(body)
            mask.append(tok + _blank(body[len(tok):-1]) + body[-1:])
        else:  # a char literal ('x', '\n', '\u{1F600}') or a lifetime ('a)
            cm = re.compile(r"'(\\(?:u\{[0-9a-fA-F]+\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])'").match(src, s)
            if cm:
                j = cm.end()
                code.append(src[s:j])
                mask.append("'" + " " * (j - s - 2) + "'")
            else:
                j = s + 1
                code.append("'")
                mask.append("'")
        i = j
    return "".join(code), "".join(mask)


def _line_starts(text: str) -> list[int]:
    starts = [0]
    starts.extend(m.end() for m in re.finditer("\n", text))
    return starts


def _declared_module(lines: list[str], i: int) -> str | None:
    """The `x` of a `mod x;` gated by the `#[cfg(test)]` on line index `i`."""
    rest = lines[i].split("#[cfg(test)]", 1)[-1]
    for nxt in [rest] + lines[i + 1:]:
        m = _MOD_DECL.match(nxt)
        if m:
            return m.group(1)
        if not _SKIPPABLE.match(nxt):
            return None
    return None


def _item_end(mask: str, start: int) -> int:
    """Offset just past the item that begins at `start`: its matched `{ … }`, or its `;`."""
    depth = 0
    for j in range(start, len(mask)):
        ch = mask[j]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return j + 1
        elif ch == ";" and depth == 0:
            return j + 1
    return len(mask)


def cfg_test_ranges(mask: str) -> list[tuple[int, int]]:
    """Inclusive (first, last) line numbers of every `#[cfg(test)]` item in a lexed file."""
    starts = _line_starts(mask)
    lines = mask.split("\n")
    out = []
    for m in _CFG_TEST.finditer(mask):
        first = bisect.bisect_right(starts, m.start())
        if _declared_module(lines, first - 1):
            continue
        end = _item_end(mask, m.end())
        out.append((first, bisect.bisect_right(starts, max(end - 1, m.start()))))
    return out


def is_test_file_by_name(rel: str) -> bool:
    """A file that is test code by its name or directory alone."""
    rel = "/" + rel.replace("\\", "/")
    return rel.endswith(TEST_FILE_SUFFIXES) or any(p in rel for p in TEST_DIR_PARTS)


def _child_module_dir(declaring: pathlib.Path) -> pathlib.Path:
    if declaring.name in ("mod.rs", "lib.rs", "main.rs"):
        return declaring.parent
    return declaring.parent / declaring.stem


def cfg_test_module_paths(files) -> list[pathlib.Path]:
    """Every `x.rs` / `x/` declared by a `#[cfg(test)] mod x;` in `files`.

    The declaration may be separated from its attribute by other attributes or
    blank lines.
    """
    out: list[pathlib.Path] = []
    for f in files:
        lines = pathlib.Path(f).read_text(errors="replace").splitlines()
        for i, line in enumerate(lines):
            if not _CFG_TEST.match(line):
                continue
            name = _declared_module(lines, i)
            if name:
                base = _child_module_dir(pathlib.Path(f))
                out.append(base / f"{name}.rs")
                out.append(base / name)
    return out


class TestCode:
    """Answers "is line N of file F test code?" over one set of Rust files."""

    def __init__(self, root: pathlib.Path, files):
        self.root = root.resolve()
        declared = [str(p.resolve()) for p in cfg_test_module_paths(files)]
        self._declared = tuple(declared) + tuple(d + "/" for d in declared)
        self._ranges: dict[pathlib.Path, list[tuple[int, int]]] = {}

    def is_test_file(self, path: pathlib.Path) -> bool:
        resolved = pathlib.Path(path).resolve()
        if is_test_file_by_name(str(resolved.relative_to(self.root))):
            return True
        s = str(resolved)
        return s in self._declared or s.startswith(self._declared)

    def is_test_line(self, path: pathlib.Path, lineno: int, mask: str | None = None) -> bool:
        """`mask` is the file's lexed mask when the caller already has it."""
        path = pathlib.Path(path)
        if self.is_test_file(path):
            return True
        if path not in self._ranges:
            if mask is None:
                mask = lex_rust(path.read_text(errors="replace"))[1]
            self._ranges[path] = cfg_test_ranges(mask)
        return any(a <= lineno <= b for a, b in self._ranges[path])
