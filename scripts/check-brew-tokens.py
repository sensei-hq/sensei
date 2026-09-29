#!/usr/bin/env python3
"""Every `brew …` command in the docs must name a tap entry that exists.

A tap resolves `sensei-hq/tap/<token>` by FILENAME, and it resolves the formula
namespace and the cask namespace separately. Two consequences that reading the
docs cannot reveal:

  * A token with no file is a hard error — but only on a user's machine.
  * `brew install sensei-hq/tap/X` with no `--cask` falls THROUGH to the casks
    when no formula named X exists. So a command naming the wrong namespace can
    install the wrong artifact and still look like it worked.

Both were live in `README.md`: the install block named `senseihq` for the CLI and
`sensei` for the desktop app — the two tokens exactly transposed. The first
silently installed the app cask; the second resolved a dead `Casks/sensei.rb`
left behind by a rename and 404'd on a `v0.1.0` URL.

The tokens are read from `homebrew/` — the templates this repo owns and
`scripts/render-homebrew-tap.py` publishes — so the gate tracks a rename
automatically instead of restating a list that drifts.

Usage:
    check-brew-tokens.py            check the tree, exit 1 on findings
    check-brew-tokens.py --self-test
"""
from __future__ import annotations

import pathlib
import re
import subprocess
import sys

FORMULA_DIR = "homebrew/Formula"
CASK_DIR = "homebrew/Casks"

# `brew <verb> [--cask|--formula] sensei-hq/tap/<token>`. The flags brew accepts
# between the verb and the argument are captured so the namespace can be checked.
COMMAND = re.compile(
    r"brew\s+(?P<verb>install|reinstall|upgrade|info|fetch|uninstall|services)"
    r"(?P<mid>(?:\s+\S+)*?)\s+sensei-hq/tap/(?P<token>[a-z0-9][a-z0-9-]*)"
)
CASK_TOKEN = re.compile(r'^cask\s+"(?P<token>[^"]+)"', re.M)

# A fixture is frozen INPUT to a test. Editing one to satisfy a docs gate changes
# what that test measures, so the gate must not reach into them.
SKIP = ("/fixtures/",)


def tap_tokens(root: pathlib.Path) -> tuple[set[str], set[str], list[str]]:
    """(formula tokens, cask tokens, problems) as the templates declare them."""
    problems: list[str] = []

    formulae = {p.stem for p in (root / FORMULA_DIR).glob("*.rb")}

    casks: set[str] = set()
    for path in (root / CASK_DIR).glob("*.rb"):
        casks.add(path.stem)
        declared = CASK_TOKEN.search(path.read_text(errors="replace"))
        # brew loads `Casks/<stem>.rb` and then requires the block inside to
        # declare that same token. They disagreeing is an error at load time, on
        # the user's machine, for a file that reads fine here.
        if declared and declared.group("token") != path.stem:
            problems.append(
                f"{CASK_DIR}/{path.name}: declares cask \"{declared.group('token')}\" "
                f"but brew resolves this file as \"{path.stem}\""
            )

    return formulae, casks, problems


def docs() -> list[pathlib.Path]:
    listed = subprocess.run(
        ["git", "ls-files", "*.md"], capture_output=True, text=True
    ).stdout.split()
    return [pathlib.Path(f) for f in listed if not any(s in f"/{f}" for s in SKIP)]


def check_line(
    line: str, formulae: set[str], casks: set[str]
) -> list[str]:
    """Every tap reference on one line, with what is wrong about it."""
    found: list[str] = []

    for m in COMMAND.finditer(line):
        token, verb, mid = m.group("token"), m.group("verb"), m.group("mid")
        wants_cask = "--cask" in mid
        wants_formula = "--formula" in mid or verb == "services"

        if wants_cask and token not in casks:
            found.append(f"--cask names {token!r}, which is not a cask in {CASK_DIR}/")
        elif wants_formula and token not in formulae:
            kind = "services" if verb == "services" else "--formula"
            found.append(f"{kind} names {token!r}, which is not a formula in {FORMULA_DIR}/")
        elif not wants_cask and not wants_formula and token not in formulae:
            # Not merely missing: brew falls through to the cask namespace, so
            # this INSTALLS something — just not the thing the line claims.
            extra = " (brew would fall through and install the cask)" if token in casks else ""
            found.append(f"{token!r} is not a formula in {FORMULA_DIR}/{extra}")

    return found


def findings(root: pathlib.Path) -> tuple[list[str], int]:
    formulae, casks, problems = tap_tokens(root)
    out = list(problems)
    scanned = 0

    for path in docs():
        for i, line in enumerate(path.read_text(errors="replace").split("\n"), start=1):
            if "sensei-hq/tap/" not in line:
                continue
            scanned += 1
            for problem in check_line(line, formulae, casks):
                out.append(f"{path}:{i}: {problem}")

    return out, scanned


def self_test() -> int:
    formulae, casks = {"sensei"}, {"senseihq"}
    cases = [
        ("the formula, named correctly", "brew install sensei-hq/tap/sensei", 0),
        ("the cask, named correctly", "brew install --cask sensei-hq/tap/senseihq", 0),
        ("tokens transposed — the live README bug", "brew install sensei-hq/tap/senseihq", 1),
        ("a cask that does not exist", "brew install --cask sensei-hq/tap/sensei-app", 1),
        ("services on a non-formula", "brew services start sensei-hq/tap/senseihq", 1),
        ("services on the formula", "brew services start sensei-hq/tap/sensei", 0),
        ("both wrong on one line", "brew install sensei-hq/tap/nope && brew install --cask sensei-hq/tap/nope", 2),
        ("a tap command with no token", "brew tap sensei-hq/tap", 0),
    ]

    passed = 0
    for name, line, want in cases:
        got = len(check_line(line, formulae, casks))
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

    root = pathlib.Path(__file__).resolve().parent.parent
    problems, scanned = findings(root)

    if not problems:
        print(f"check-brew-tokens: {scanned} tap reference(s), every one names a real entry.")
        return 0

    print(f"check-brew-tokens: {len(problems)} bad tap reference(s)\n")
    for problem in problems:
        print(f"  {problem}")
    print(f"\nTokens the templates declare: formulae {sorted(tap_tokens(root)[0])} "
          f"· casks {sorted(tap_tokens(root)[1])}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
