#!/usr/bin/env python3
"""Every dependency app and the dōjō both declare must carry the same spec.

The two are separate bun installs with separate lockfiles, so nothing makes a
shared dependency agree. They drifted: vitest ^4.1.5 vs ^4, jsdom ^29 vs ^25,
svelte-check ^4.4.8 vs ^4.4.6, and the Libraries screen showed each spread as a
version conflict in the workspace (audit 2026-11-03, decision 1b). One spec per
shared name is the invariant; a lockfile refresh then converges the resolved
versions too.

A deliberate difference is recorded in `scripts/shared-js-specs-allow.tsv`
(dependency <TAB> reason). An entry that no longer differs is itself a finding,
so the list cannot outlive the reason it was written for.

The section is not compared: one app shipping a package as a dependency and the
other as a devDependency is a bundling choice, not a version disagreement.

Usage:
    check-shared-js-specs.py            check the tree, exit 1 on findings
    check-shared-js-specs.py --self-test
"""
from __future__ import annotations

import json
import pathlib
import sys

MANIFESTS = ("app/package.json", "dojo/package.json")
ALLOW = "scripts/shared-js-specs-allow.tsv"
SECTIONS = ("dependencies", "devDependencies", "peerDependencies", "optionalDependencies")


def specs(manifest: dict) -> dict[str, set[str]]:
    """Every spec a manifest gives each dependency, across all sections."""
    out: dict[str, set[str]] = {}
    for section in SECTIONS:
        for name, spec in manifest.get(section, {}).items():
            out.setdefault(name, set()).add(spec)
    return out


def read_allow(text: str) -> dict[str, str]:
    """dependency -> reason. Blank lines and `#` comments are skipped."""
    allow: dict[str, str] = {}
    for line in text.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        name, _, reason = line.partition("\t")
        allow[name.strip()] = reason.strip()
    return allow


def findings(
    a_name: str, a: dict, b_name: str, b: dict, allow: dict[str, str]
) -> tuple[list[str], int]:
    """(problems, number of shared dependencies compared)."""
    sa, sb = specs(a), specs(b)
    shared = sorted(sa.keys() & sb.keys())
    problems: list[str] = []

    for name in shared:
        if sa[name] == sb[name] and len(sa[name]) == 1:
            if name in allow:
                problems.append(
                    f"{name}: allowlisted, but both declare {sorted(sa[name])[0]!r} — remove it from {ALLOW}"
                )
            continue
        if name in allow:
            continue
        problems.append(
            f"{name}: {a_name} {' | '.join(sorted(sa[name]))} vs {b_name} {' | '.join(sorted(sb[name]))}"
        )

    for name in sorted(allow.keys() - set(shared)):
        problems.append(f"{name}: allowlisted, but not declared by both — remove it from {ALLOW}")

    return problems, len(shared)


def self_test() -> int:
    def m(**deps: str) -> dict:
        return {"devDependencies": deps}

    cases = [
        ("identical specs", m(vitest="^4.1.12"), m(vitest="^4.1.12"), {}, 0),
        ("the live drift: jsdom ^29 vs ^25", m(jsdom="^29.1.1"), m(jsdom="^25.0.1"), {}, 1),
        ("a bare major vs a floor", m(vitest="^4.1.5"), m(vitest="^4"), {}, 1),
        ("a dependency only one side has", m(wrangler="^4"), m(d3="^3"), {}, 0),
        (
            "same spec, different sections",
            {"dependencies": {"@iconify-json/simple-icons": "^1.2.91"}},
            m(**{"@iconify-json/simple-icons": "^1.2.91"}),
            {},
            0,
        ),
        (
            "different spec, different sections",
            {"dependencies": {"svelte": "^5.56.4"}},
            m(svelte="^5.55.4"),
            {},
            1,
        ),
        (
            "one manifest disagreeing with itself",
            {"dependencies": {"svelte": "^5.56.4"}, "devDependencies": {"svelte": "^5.55.4"}},
            m(svelte="^5.56.4"),
            {},
            1,
        ),
        ("an allowlisted difference", m(typescript="5.9.3"), m(typescript="^6.0.3"), {"typescript": "engine"}, 0),
        ("a stale allow entry (they agree now)", m(vite="^8"), m(vite="^8"), {"vite": "old"}, 1),
        ("a stale allow entry (no longer shared)", m(vite="^8"), m(), {"vite": "old"}, 1),
        ("two drifts are two findings", m(a="1", b="1"), m(a="2", b="2"), {}, 2),
    ]

    passed = 0
    for name, a, b, allow, want in cases:
        got = len(findings("app", a, "dojo", b, allow)[0])
        if got == want:
            passed += 1
            print(f"  ok    {name}")
        else:
            print(f"  FAIL  {name}: found {got}, expected {want}")

    text = "# comment\n\njsdom\ttest env only\n"
    if read_allow(text) == {"jsdom": "test env only"}:
        passed += 1
        print("  ok    allowlist parsing skips comments and blanks")
    else:
        print(f"  FAIL  allowlist parsing: {read_allow(text)!r}")

    total = len(cases) + 1
    print(f"  {passed}/{total} passed")
    return 0 if passed == total else 1


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()

    root = pathlib.Path(__file__).resolve().parent.parent
    (a_path, b_path) = MANIFESTS
    a = json.loads((root / a_path).read_text())
    b = json.loads((root / b_path).read_text())
    allow = read_allow((root / ALLOW).read_text())

    problems, shared = findings(a_path, a, b_path, b, allow)
    if not problems:
        print(f"check-shared-js-specs: {shared} shared dependencies, one spec each.")
        return 0

    print(f"check-shared-js-specs: {len(problems)} finding(s) over {shared} shared dependencies\n")
    for problem in problems:
        print(f"  {problem}")
    print(f"\nAlign the specs, or record a deliberate difference in {ALLOW}.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
