#!/usr/bin/env python3
"""Render the Homebrew formula and cask into a tap checkout.

The monorepo's `homebrew/` copies are TEMPLATES: they carry the version that
`make bump` stamped and `REPLACE_WITH_*_SHA256` where each checksum goes, because
a checksum cannot exist until the release assets do. This renders them — version
plus real SHA256s — into a tap working copy, and REFUSES to write anything it
cannot fully account for.

WHY THIS EXISTS AS A SCRIPT. It was inline `sed`+`awk` in `update-tap`, which made
it untestable, and the two defects below are exactly the kind that inline shell in
a release-only job hides until someone tries to install:

1. **Fail-open publishing.** `make bump` used to push the template straight to the
   tap and rely on `update-tap` to repair it minutes later. When
   `TAP_GITHUB_TOKEN` expired, the repair stopped and the tap was left ADVERTISING
   a version whose every checksum was the literal string
   `REPLACE_WITH_ARM64_SHA256`. `brew install` could not work for ~8 releases
   (v0.7.3 … v0.10.0) and nothing said so, because the last good formula had
   already been overwritten. The tap now only ever receives a rendered file, so a
   failure anywhere leaves users on the previous working release.

2. **Positional platform matching.** The old `awk` decided which checksum to write
   by tracking `if OS.mac? && Hardware::CPU.arm?` / `elsif` / `else` lines and
   assigning to whichever `sha256` came next. That is correct only while the
   conditional order never changes, and it fails SILENTLY when it does — writing a
   valid-looking checksum for the wrong artifact. A wrong-but-well-formed SHA
   fails as a MISMATCH, which reads like a corrupted download or a tampered
   asset, and is strictly harder to diagnose than a placeholder. This keys each
   checksum off the ARTIFACT NAME in the URL directly above it, so a reordered or
   newly added platform block cannot mis-assign — it can only fail to match, and
   an unmatched URL is an error.

Usage:
    render-homebrew-tap.py <tap-dir> <version>

Checksums come from the environment so they never reach `argv` (and therefore
never reach `ps` or a process-accounting log):

    SHA_MACOS_ARM64, SHA_MACOS_X86_64, SHA_LINUX_ARM64, SHA_LINUX_X86_64
    SHA_DMG   — optional; when absent the cask is left untouched

Exit 0 only when every URL in the rendered formula carries the checksum belonging
to the artifact it names, and no placeholder survives.
"""

import os
import re
import sys
from pathlib import Path

# Artifact filename -> the env var holding its checksum. The filename is what
# appears in the formula's `url`, which is what makes the mapping positional-free.
ARTIFACTS = {
    "sensei-macos-arm64.tar.gz": "SHA_MACOS_ARM64",
    "sensei-macos-x86_64.tar.gz": "SHA_MACOS_X86_64",
    "sensei-linux-arm64.tar.gz": "SHA_LINUX_ARM64",
    "sensei-linux-x86_64.tar.gz": "SHA_LINUX_X86_64",
}

SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
VERSION_LINE = re.compile(r'^(?P<indent>\s*)version "(?P<old>[^"]*)"', re.MULTILINE)
# A `url` line followed — not necessarily immediately — by the `sha256` it owns.
URL_THEN_SHA = re.compile(
    r'url "(?P<url>[^"]*)"(?P<between>(?:\s*#[^\n]*\n)*\s*)sha256 "(?P<sha>[^"]*)"'
)


def die(msg: str) -> "NoReturn":  # type: ignore[valid-type]
    print(f"render-homebrew-tap: {msg}", file=sys.stderr)
    raise SystemExit(1)


def read_checksums() -> dict[str, str]:
    """Every artifact's checksum, validated as a SHA256 before it is written.

    A truncated or error-text value would otherwise be committed verbatim and only
    surface as a mismatch on a user's machine.
    """
    out: dict[str, str] = {}
    for artifact, var in ARTIFACTS.items():
        value = (os.environ.get(var) or "").strip()
        if not value:
            die(f"{var} is empty — refusing to render a formula with a missing checksum")
        if not SHA256_RE.match(value):
            die(f"{var} is not a SHA256 ({len(value)} chars): {value[:24]}…")
        out[artifact] = value
    return out


def set_version(text: str, version: str, path: Path) -> str:
    new, count = VERSION_LINE.subn(lambda m: f'{m.group("indent")}version "{version}"', text, count=1)
    if count != 1:
        die(f"{path}: expected exactly one `version \"…\"` line, replaced {count}")
    return new


def fill_formula(text: str, shas: dict[str, str], path: Path) -> str:
    """Write each checksum next to the URL naming its artifact."""
    seen: set[str] = set()

    def one(m: re.Match[str]) -> str:
        url = m.group("url")
        artifact = next((a for a in ARTIFACTS if url.endswith(a)), None)
        if artifact is None:
            die(f"{path}: url names no known artifact, so no checksum can be chosen: {url}")
        seen.add(artifact)
        return f'url "{url}"{m.group("between")}sha256 "{shas[artifact]}"'

    rendered = URL_THEN_SHA.sub(one, text)
    missing = set(ARTIFACTS) - seen
    if missing:
        die(f"{path}: no url found for {sorted(missing)} — the formula lost a platform block")
    return rendered


def verify(text: str, shas: dict[str, str], path: Path) -> None:
    """Re-derive the mapping from the RENDERED text rather than trusting the write."""
    if "REPLACE_WITH_" in text:
        leftover = sorted(set(re.findall(r"REPLACE_WITH_[A-Z0-9_]+", text)))
        die(f"{path}: placeholder(s) survived rendering: {leftover}")
    pairs = {
        next((a for a in ARTIFACTS if m.group("url").endswith(a)), m.group("url")): m.group("sha")
        for m in URL_THEN_SHA.finditer(text)
    }
    for artifact, expected in shas.items():
        got = pairs.get(artifact)
        if got != expected:
            die(
                f"{path}: {artifact} carries the wrong checksum — "
                f"expected {expected[:12]}…, found {(got or '<none>')[:12]}…"
            )
    print(f"render-homebrew-tap: {path.name} — {len(pairs)} url/checksum pairs, all correct.")


def main() -> int:
    if len(sys.argv) != 3:
        die("usage: render-homebrew-tap.py <tap-dir> <version>")
    tap = Path(sys.argv[1])
    version = sys.argv[2].lstrip("v")
    if not version:
        die("version is empty")

    root = Path(__file__).resolve().parent.parent
    formula_src = root / "homebrew" / "Formula" / "sensei.rb"
    cask_src = root / "homebrew" / "Casks" / "senseihq.rb"
    for p in (formula_src, cask_src):
        if not p.is_file():
            die(f"template not found: {p}")
    for d in (tap / "Formula", tap / "Casks"):
        if not d.is_dir():
            die(f"not a tap checkout — missing {d}")

    shas = read_checksums()

    formula = set_version(formula_src.read_text(), version, formula_src)
    formula = fill_formula(formula, shas, formula_src)
    verify(formula, shas, formula_src)
    (tap / "Formula" / "sensei.rb").write_text(formula)

    # The cask is OPTIONAL: a release without a DMG is a valid release (the
    # desktop build is a separate job), and rendering the cask with no checksum
    # would publish a broken one. Leaving the tap's existing cask alone keeps the
    # previous working version installable — the same fail-closed rule as above.
    dmg = (os.environ.get("SHA_DMG") or "").strip()
    if not dmg:
        print("render-homebrew-tap: no SHA_DMG — cask left at its current version.")
    elif not SHA256_RE.match(dmg):
        die(f"SHA_DMG is not a SHA256 ({len(dmg)} chars)")
    else:
        cask = set_version(cask_src.read_text(), version, cask_src)
        cask, n = re.subn(r'sha256 "[^"]*"', f'sha256 "{dmg}"', cask, count=1)
        if n != 1:
            die(f"{cask_src}: expected exactly one `sha256 \"…\"`, replaced {n}")
        if "REPLACE_WITH_" in cask:
            die(f"{cask_src}: placeholder survived rendering")
        (tap / "Casks" / "senseihq.rb").write_text(cask)
        print("render-homebrew-tap: senseihq.rb — version and checksum written.")

    print(f"render-homebrew-tap: rendered v{version} into {tap}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
