#!/usr/bin/env python3
"""Every relative link in the docs points at a file or folder git tracks.

A link into an untracked folder (docs/mockups/ lives only on one machine) or at a
file that moved is dead for everyone else and on GitHub, and nothing else notices.
External links (scheme:) and same-page anchors (#…) are not checked.

Known dead links that another change is about to fix live in
scripts/doc-links-allow.tsv (file<TAB>target). A row whose link no longer exists
fails, so the list can only shrink.

  python3 scripts/check-doc-links.py          check the tree
  python3 scripts/check-doc-links.py --test   self-check on a temporary tree
"""
import os
import re
import subprocess
import sys
import tempfile
import urllib.parse

sys.dont_write_bytecode = True
LINK = re.compile(r'!?\[[^\]]*\]\(([^)\s]+)\)')
ALLOW = "scripts/doc-links-allow.tsv"


def tracked_paths(root, env=None):
    out = subprocess.run(["git", "ls-files"], cwd=root, capture_output=True, text=True, check=True, env=env).stdout
    files = {p for p in out.split("\n") if p}
    dirs = set()
    for f in files:
        d = os.path.dirname(f)
        while d:
            dirs.add(d)
            d = os.path.dirname(d)
    return files, dirs


def dead_links(root, env=None):
    files, dirs = tracked_paths(root, env)
    dead = []
    for f in sorted(files):
        if not f.endswith(".md") or not (f.startswith("docs/") or "/" not in f):
            continue
        with open(os.path.join(root, f), encoding="utf-8", errors="ignore") as fh:
            text = fh.read()
        for m in LINK.finditer(text):
            url = m.group(1)
            if re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*:", url) or url.startswith("#"):
                continue
            path = urllib.parse.unquote(url.split("#", 1)[0])
            if not path:
                continue
            target = os.path.normpath(os.path.join(os.path.dirname(f), path))
            if target not in files and target not in dirs:
                dead.append((f, url))
    return dead


def load_allow(root):
    p = os.path.join(root, ALLOW)
    if not os.path.exists(p):
        return set()
    rows = set()
    for line in open(p, encoding="utf-8"):
        line = line.rstrip("\n")
        if line and not line.startswith("#"):
            f, _, url = line.partition("\t")
            rows.add((f, url))
    return rows


def check(root, env=None):
    dead = set(dead_links(root, env))
    allow = load_allow(root)
    new = sorted(dead - allow)
    stale = sorted(allow - dead)
    for f, url in new:
        print(f"check-doc-links: dead link in {f}: {url}")
    for f, url in stale:
        print(f"check-doc-links: {ALLOW} row is fixed now, remove it: {f}\t{url}")
    if new or stale:
        print("check-doc-links: link only to files git tracks; drop the link (keep its text) when the target is gone.")
        return 1
    print(f"check-doc-links: no dead links ({len(allow)} allowlisted, pending a fix).")
    return 0


def self_test():
    with tempfile.TemporaryDirectory() as d:
        # Under a pre-commit hook git exports GIT_DIR/GIT_INDEX_FILE; a child git that
        # inherits them acts on the REAL repository (its index, its config), not this
        # temporary one. Strip every GIT_* variable for the temporary repo.
        env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}

        def git(*a):
            subprocess.run(["git", *a], cwd=d, check=True, capture_output=True, env=env)
        git("init", "-q")
        os.makedirs(os.path.join(d, "docs", "sub"))
        os.makedirs(os.path.join(d, "scripts"))
        open(os.path.join(d, "docs", "a.md"), "w").write(
            "[ok](sub/b.md) [dir](sub) [web](https://x.y) [anchor](#h) [dead](../mockups/m.html) [dead2](gone.md#x)\n")
        open(os.path.join(d, "docs", "sub", "b.md"), "w").write("[up](../a.md)\n")
        git("add", "-A")
        got = sorted(dead_links(d, env))
        want = [("docs/a.md", "../mockups/m.html"), ("docs/a.md", "gone.md#x")]
        assert got == want, f"dead_links: {got} != {want}"
        assert check(d, env) == 1, "a dead link must fail"
        open(os.path.join(d, ALLOW), "w").write("docs/a.md\t../mockups/m.html\ndocs/a.md\tgone.md#x\n")
        assert check(d, env) == 0, "allowlisted dead links pass"
        open(os.path.join(d, "docs", "a.md"), "w").write("[ok](sub/b.md)\n")
        assert check(d, env) == 1, "a stale allow row must fail"
    print("check-doc-links --test: ok")
    return 0


if __name__ == "__main__":
    root = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True).stdout.strip()
    sys.exit(self_test() if "--test" in sys.argv else check(root))
