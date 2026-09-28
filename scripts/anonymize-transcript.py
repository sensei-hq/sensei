#!/usr/bin/env python3
"""Turn a real assistant transcript into a committable test fixture.

WHY THIS EXISTS. The transcript adapters are tested entirely from inline Rust
string literals — 93 tests, 109 literals, zero file fixtures — because the leak
guard blocks every transcript path in the repo. That is safe, but the literals
are hand-written approximations of formats that change without notice: a tool
ships a schema tweak and every test stays green while ingestion breaks.

The fix is a fixture captured from the real thing. The fix is NOT hand-copying a
session into the repo, which is precisely the move that put a contributor's
username and their employer's project path into a public repo and cost a history
rewrite. So fixtures are machine-generated, and this is the machine.

THE RULE: PRESERVE SHAPE, REPLACE CONTENT.

  preserved (a parser depends on it)   replaced (it can carry anything)
  ----------------------------------   --------------------------------
  every JSON key                       filesystem paths
  every number (tokens, costs, ms)     email addresses
  booleans and nulls                   URLs (host and path)
  array order and nesting depth        uuids and session ids
  identifier-like strings: role,       free prose and code bodies
    type, tool names, model names      (length, line count and code-fence
  relative timing between events         structure are kept; the words are not)

FREE TEXT IS REPLACED, NOT SCRUBBED. A prompt or a model response can contain
literally anything — a client name, a password, a colleague's address — and no
denylist can be trusted to find all of it. Scrubbing free prose is a bet that
you enumerated every secret in it. Replacing it is not a bet. For a parser test
the words are irrelevant anyway; the structure is the whole subject.

REPLACEMENT IS DETERMINISTIC AND REFERENTIALLY CONSISTENT. The same input string
always yields the same pseudonym, so a repo that appears in forty turns appears
as ONE fake repo in forty turns. Without that, a test asserting "these turns
share a cwd" would pass on the real transcript and fail on the fixture, and the
fixture would be silently useless.

TIMESTAMPS ARE SHIFTED, NOT ZEROED. All instants move by one constant offset, so
durations and ordering survive while the actual date does not.

Usage:
    anonymize-transcript.py <input> --family claude --case tool-use
    anonymize-transcript.py <input> --family zed --case subagent --tool-version 0.164.1
    anonymize-transcript.py --self-test
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import pathlib
import re
import sys

FIXTURE_ROOT = pathlib.Path("crates/senseid/tests/fixtures/transcripts")
ANONYMISER_VERSION = "1"

# Synthetic vocabularies. Small and obviously fake on sight — a reader must never
# have to wonder whether a fixture name is somebody's real project.
USERS = ["dev"]
ORGS = ["example-org", "demo-labs", "sample-co"]
REPOS = ["widget-api", "ledger-ui", "atlas-worker", "beacon-cli", "harbor-svc",
         "kestrel-lib", "moss-tool", "nimbus-web"]
DIRS = ["src", "lib", "app", "core", "tests", "docs", "pkg", "internal"]
FILES = ["main", "handler", "client", "model", "router", "store", "helper", "config"]
WORDS = ["alpha", "bravo", "charlie", "delta", "echo", "foxtrot", "golf", "hotel",
         "india", "juliet", "kilo", "lima", "mike", "november", "oscar", "papa"]

# A string is left alone when it looks like a discriminant rather than content:
# short, no whitespace, no separators. `assistant`, `tool_use`, `Read`,
# `claude-opus-4` all survive, and a parser keyed on any of them keeps working.
IDENTIFIER = re.compile(r"^[A-Za-z][A-Za-z0-9._\-]{0,48}$")
PATHISH = re.compile(r"(^|[\s\"'(])(/[^\s\"']{2,}|[A-Za-z]:\\[^\s\"']{2,}|~/[^\s\"']{2,})")
EMAIL = re.compile(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}")
URL = re.compile(r"https?://[^\s\"'<>)]+")
UUID = re.compile(r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-"
                  r"[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b")
HEXID = re.compile(r"\b[0-9a-fA-F]{24,64}\b")
ISO_TS = re.compile(r"\b\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+\-]\d{2}:?\d{2})?")

# Keys whose STRING value is a discriminant the parser switches on. Never touched
# even if the value happens to look like prose.
PRESERVE_KEYS = {
    "role", "type", "kind", "status", "state", "event", "event_type", "subtype",
    "model", "family", "source", "provider", "tool", "tool_name", "name",
    "stop_reason", "finish_reason", "level", "version", "schema", "mime_type",
}
# Keys that are free text no matter what they contain.
TEXT_KEYS = {"text", "content", "message", "prompt", "response", "body", "summary",
             "description", "thinking", "output", "stdout", "stderr", "command",
             "input", "result", "title", "instruction", "reasoning"}

TIME_SHIFT = dt.timedelta(days=-419, hours=-7, minutes=-13)


def _pick(bucket: list[str], seed: str) -> str:
    """Deterministic choice: the same seed always yields the same element."""
    h = int(hashlib.sha256(seed.encode("utf-8", "replace")).hexdigest()[:12], 16)
    return bucket[h % len(bucket)]


def fake_uuid(seed: str) -> str:
    h = hashlib.sha256(("uuid" + seed).encode()).hexdigest()
    return f"{h[0:8]}-{h[8:12]}-4{h[13:16]}-8{h[17:20]}-{h[20:32]}"


def fake_hexid(seed: str, length: int) -> str:
    out = ""
    i = 0
    while len(out) < length:
        out += hashlib.sha256(f"hex{i}{seed}".encode()).hexdigest()
        i += 1
    return out[:length]


def fake_path(original: str) -> str:
    """A synthetic path with the same DEPTH and extension as the original.

    Depth is preserved because adapters derive a project root and a repo name by
    walking path segments, and a fixture whose paths are all one level deep would
    exercise none of that.
    """
    win = bool(re.match(r"^[A-Za-z]:\\", original))
    raw = original.replace("\\", "/")
    segs = [s for s in raw.split("/") if s]
    ext = ""
    if segs and "." in segs[-1]:
        ext = "." + segs[-1].rsplit(".", 1)[1]
        if len(ext) > 8 or not ext[1:].isalnum():
            ext = ""
    depth = max(len(segs), 3)
    user = _pick(USERS, original)
    repo = _pick(REPOS, original)
    out = ["", "Users", user, "Projects", repo]
    for i in range(max(0, depth - 5)):
        out.append(_pick(DIRS, f"{original}/d{i}"))
    leaf = _pick(FILES, original) + ext
    out.append(leaf)
    p = "/".join(out)
    return "C:\\Users\\" + user + "\\Projects\\" + repo + "\\" + leaf if win else p


def fake_prose(original: str) -> str:
    """Filler with the original's line count and rough length, and its code
    fences kept — a parser that splits on ``` or counts lines still sees them."""
    lines = original.split("\n")
    out = []
    for idx, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("```"):
            out.append(line)  # fence markers are structure
            continue
        if not stripped:
            out.append("")
            continue
        n = max(1, len(stripped) // 6)
        words = [_pick(WORDS, f"{original[:32]}:{idx}:{w}") for w in range(min(n, 24))]
        out.append(" ".join(words))
    return "\n".join(out)


def shift_timestamp(s: str) -> str:
    try:
        norm = s.replace("Z", "+00:00")
        parsed = dt.datetime.fromisoformat(norm)
    except ValueError:
        return s
    shifted = parsed + TIME_SHIFT
    return shifted.isoformat().replace("+00:00", "Z") if s.endswith("Z") else shifted.isoformat()


def scrub_string(value: str, key: str | None) -> str:
    if key in PRESERVE_KEYS:
        return value
    if ISO_TS.fullmatch(value.strip()):
        return shift_timestamp(value.strip())
    if UUID.fullmatch(value.strip()):
        return fake_uuid(value)
    # A text key is content, whatever its length. The first version of this gated
    # on `len > 80 or contains a newline`, and a 27-character prompt sailed
    # through untouched — a short prompt names a client as readily as a long one.
    if key in TEXT_KEYS:
        return fake_prose(value)
    if IDENTIFIER.fullmatch(value) and "/" not in value and "\\" not in value:
        return value
    # Ordinary string: rewrite the parts that can carry content, leave the rest.
    out = UUID.sub(lambda m: fake_uuid(m.group(0)), value)
    out = HEXID.sub(lambda m: fake_hexid(m.group(0), len(m.group(0))), out)
    out = EMAIL.sub("dev@example.com", out)
    out = URL.sub(lambda m: "https://example.com/" + _pick(WORDS, m.group(0)), out)
    out = ISO_TS.sub(lambda m: shift_timestamp(m.group(0)), out)
    out = PATHISH.sub(lambda m: m.group(1) + fake_path(m.group(2)), out)
    if len(out) > 80 or "\n" in out:
        return fake_prose(out)
    return out


def scrub_key(k: str) -> str:
    """A schema key is an identifier and is preserved; anything else is a MAP KEY
    and is content.

    Claude Code's `trackedFileBackups` is an object keyed by absolute file path.
    Preserving every key unconditionally — which is what the first version did —
    wrote a real home directory, and a client name inside it, into the first
    fixture this script generated. Sensei's own `props.occurrences` has the same
    shape, so this is a category, not one vendor's quirk.
    """
    return k if IDENTIFIER.fullmatch(k) else scrub_string(k, None)


def walk(node, key=None):
    if isinstance(node, dict):
        return {scrub_key(k): walk(v, k) for k, v in node.items()}
    if isinstance(node, list):
        return [walk(v, key) for v in node]
    if isinstance(node, str):
        return scrub_string(node, key)
    return node  # numbers, booleans, null are shape


def anonymize_text(raw: str) -> str:
    """JSONL in, JSONL out — one object per line, order preserved."""
    out_lines = []
    for line in raw.split("\n"):
        if not line.strip():
            continue
        try:
            obj = json.loads(line)
        except json.JSONDecodeError:
            # Not JSONL. Try the whole document as one JSON value.
            return json.dumps(walk(json.loads(raw)), indent=2, ensure_ascii=False)
        out_lines.append(json.dumps(walk(obj), ensure_ascii=False))
    return "\n".join(out_lines) + "\n"


def verify(text: str) -> list[str]:
    """Check the script's OWN OUTPUT before it is written.

    The first run of this tool printed "now run check-no-leaks --audit", which
    was useless advice: `--audit` walks `git ls-files`, and a freshly written
    fixture is untracked, so it reported the tree clean over a file carrying a
    real home directory. A generator that cannot check its own output is a
    generator you have to remember to check, and the whole point here is to not
    rely on remembering.

    Fails CLOSED: the caller deletes the output on any finding.
    """
    problems = []
    for user in sorted(set(re.findall(r"/Users/([A-Za-z0-9._\-]+)", text)) |
                       set(re.findall(r"/home/([A-Za-z0-9._\-]+)", text))):
        if user not in USERS and user not in ("user", "runner"):
            problems.append(f"home directory '/Users/{user}' is not a synthetic user")
    for mail in sorted(set(EMAIL.findall(text))):
        if not mail.endswith(("@example.com", "@example.org", "@sensei-hq.com")):
            problems.append(f"email {mail!r} is not an allow-listed domain")
    names_file = pathlib.Path.home() / ".sensei" / "private-names"
    if names_file.is_file():
        collapsed = re.sub(r"[A-Za-z0-9]{32,}", "<DIGEST>", text)
        for raw in names_file.read_text().splitlines():
            n = raw.split("#", 1)[0].strip()
            if not n:
                continue
            if re.search(r"(?<![A-Za-z0-9])" + re.escape(n) + r"(?![A-Za-z0-9])",
                         collapsed, re.I):
                # The name is NOT echoed: printing it would put the leak into the
                # terminal scrollback that reads this failure.
                problems.append("a name from your private-names denylist survived")
                break
    return problems


def self_test() -> int:
    passed = total = 0

    def check(name, got, want):
        nonlocal passed, total
        total += 1
        if got == want:
            passed += 1
            print(f"  ok    {name}")
        else:
            print(f"  FAIL  {name}\n          got  {got!r}\n          want {want!r}")

    def check_pred(name, ok, detail=""):
        nonlocal passed, total
        total += 1
        if ok:
            passed += 1
            print(f"  ok    {name}")
        else:
            print(f"  FAIL  {name} {detail}")

    print("anonymize-transcript self-test")

    # Shape survives.
    check("role preserved", scrub_string("assistant", "role"), "assistant")
    check("type preserved", scrub_string("tool_use", "type"), "tool_use")
    check("model preserved", scrub_string("claude-opus-4", "model"), "claude-opus-4")
    check("tool name preserved", scrub_string("Read", "tool_name"), "Read")
    obj = {"tokens": 1234, "ok": True, "nil": None, "items": [1, 2, 3]}
    check("numbers/bools/null untouched", walk(obj), obj)

    # Content goes.
    p = scrub_string("/Users/realperson/Work/BigClient/api/main.rs", "cwd")
    check_pred("path replaced", "realperson" not in p and "BigClient" not in p, p)
    check_pred("path keeps extension", p.endswith(".rs"), p)
    check_pred("path is synthetic root", p.startswith("/Users/dev/Projects/"), p)
    check("email replaced", scrub_string("contact me at a.person@corp.com", "note"),
          "contact me at dev@example.com")
    check_pred("uuid replaced",
               scrub_string("3f2504e0-4f89-41d3-9a0c-0305e82c3301", "id")
               != "3f2504e0-4f89-41d3-9a0c-0305e82c3301")
    check_pred("uuid stays uuid-shaped",
               UUID.fullmatch(scrub_string("3f2504e0-4f89-41d3-9a0c-0305e82c3301", "id")) is not None)

    # Referential consistency — the property a fixture is useless without.
    a = scrub_string("/Users/realperson/Work/BigClient/api/main.rs", "cwd")
    b = scrub_string("/Users/realperson/Work/BigClient/api/main.rs", "path")
    check("same input -> same pseudonym", a, b)
    c = scrub_string("/Users/realperson/Work/Other/api/main.rs", "cwd")
    check_pred("different input -> different pseudonym", a != c)

    # Free text is replaced wholesale, keeping line count and fences.
    prose = "Here is my secret plan\n```rust\nfn x() {}\n```\nand the rest of it here"
    got = scrub_string(prose, "text")
    check("prose keeps line count", got.count("\n"), prose.count("\n"))
    check_pred("prose keeps fences", got.count("```") == 2, got)
    check_pred("prose words replaced", "secret" not in got and "plan" not in got, got)

    # Timestamps shift but stay ordered and keep their gap.
    t1 = scrub_string("2026-09-28T10:00:00Z", "ts")
    t2 = scrub_string("2026-09-28T10:05:00Z", "ts")
    check_pred("timestamp changed", t1 != "2026-09-28T10:00:00Z", t1)
    d1 = dt.datetime.fromisoformat(t1.replace("Z", "+00:00"))
    d2 = dt.datetime.fromisoformat(t2.replace("Z", "+00:00"))
    check("gap preserved", (d2 - d1).total_seconds(), 300.0)
    check_pred("order preserved", d1 < d2)

    # A KEY CAN BE CONTENT. Claude Code's `trackedFileBackups` is an object keyed
    # by absolute file path, so "preserve every JSON key" leaked a real home
    # directory — and a client name inside it — straight into the first fixture
    # this script produced. Schema keys are identifiers; map keys can be anything.
    keyed = {"trackedFileBackups": {"/Users/realperson/Work/BigClient/a.rs": {"n": 1}}}
    ko = walk(keyed)
    check("schema key preserved", list(ko), ["trackedFileBackups"])
    inner = list(ko["trackedFileBackups"])[0]
    check_pred("content key replaced", "realperson" not in inner and "BigClient" not in inner, inner)
    check_pred("content key stays a path", inner.startswith("/Users/dev/Projects/"), inner)
    check("value under a content key survives", ko["trackedFileBackups"][inner], {"n": 1})
    two = walk({"m": {"/Users/a/x.rs": 1, "/Users/b/y.rs": 2}})["m"]
    check("two distinct keys stay distinct", len(two), 2)

    # End to end: keys identical, values changed.
    src = json.dumps({"type": "user", "cwd": "/Users/realperson/Work/BigClient",
                      "uuid": "3f2504e0-4f89-41d3-9a0c-0305e82c3301",
                      "message": {"role": "user", "content": "my private prompt text here"},
                      "usage": {"input_tokens": 42}})
    out = anonymize_text(src)
    o = json.loads(out.strip())
    check("top-level keys identical", sorted(o), ["cwd", "message", "type", "usage", "uuid"])
    check("nested numbers survive", o["usage"]["input_tokens"], 42)
    check("discriminants survive", (o["type"], o["message"]["role"]), ("user", "user"))
    check_pred("no original content remains",
               "realperson" not in out and "BigClient" not in out and "private" not in out)

    # The output check. `--audit` walks tracked files only, so it cannot speak
    # about a fixture that was written a second ago; this is what does.
    check("verify passes clean output", verify('{"cwd":"/Users/dev/Projects/x"}'), [])
    check_pred("verify catches a real home",
               any("synthetic user" in p for p in verify('{"cwd":"/Users/realperson/x"}')))
    check_pred("verify catches a mailbox",
               any("allow-listed" in p for p in verify('{"a":"x@corp.com"}')))

    print(f"  {passed}/{total} passed")
    return 0 if passed == total else 1


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("input", nargs="?", help="a real transcript file")
    ap.add_argument("--family", help="sensei.assistant_family value, e.g. claude")
    ap.add_argument("--case", help="fixture name, e.g. tool-use")
    ap.add_argument("--tool-version", default="unknown",
                    help="version of the tool that produced the capture")
    ap.add_argument("--out-root", default=str(FIXTURE_ROOT))
    ap.add_argument("--self-test", action="store_true")
    a = ap.parse_args()

    if a.self_test:
        return self_test()
    if not (a.input and a.family and a.case):
        ap.error("input, --family and --case are required (or use --self-test)")

    src = pathlib.Path(a.input)
    raw = src.read_text(errors="replace")
    body = anonymize_text(raw)

    problems = verify(body)
    if problems:
        print("REFUSING TO WRITE — the anonymised output still carries content:",
              file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        print("\nNothing was written. This is a bug in this script, not in your "
              "input: fix the rule that missed it and re-run.", file=sys.stderr)
        return 1

    out_dir = pathlib.Path(a.out_root) / a.family
    out_dir.mkdir(parents=True, exist_ok=True)
    fixture = out_dir / f"{a.case}.jsonl"
    fixture.write_text(body)

    meta = {
        "family": a.family,
        "case": a.case,
        "tool_version": a.tool_version,
        "anonymiser_version": ANONYMISER_VERSION,
        "generated": dt.date.today().isoformat(),
        "source_bytes": len(raw),
        "fixture_sha256": hashlib.sha256(body.encode()).hexdigest(),
        "note": "Machine-generated by scripts/anonymize-transcript.py. Shape is "
                "real; every path, identifier and word is synthetic. Do not "
                "hand-edit — regenerate.",
    }
    (out_dir / f"{a.case}.meta.json").write_text(json.dumps(meta, indent=2) + "\n")

    print(f"wrote {fixture}")
    print(f"wrote {out_dir / (a.case + '.meta.json')}")
    print(f"verified: {len(body.encode())} bytes, no real home, mailbox or "
          f"denylisted name survived")
    return 0


if __name__ == "__main__":
    sys.exit(main())
