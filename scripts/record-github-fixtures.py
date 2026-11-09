#!/usr/bin/env python3
"""Record the GitHub API responses the GitHub tracker adapter's tests run on (spec
docs/spec/daemon/tracker-github.md, Build order 1). Real responses, never
hand-written: the spec's Wrong gate.

    GITHUB_TOKEN=$(gh auth token) scripts/record-github-fixtures.py \\
        --owner <org> --project <n> --no-field-project <n> --repo <name> \\
        --on <issue n> --off <issue n> --pr <pr n> \\
        --out crates/senseid/tests/fixtures/github

The token is read from the environment only and never written. The queries are
the adapter's own documents (crates/senseid/src/tracker/github_queries/), so the
fixtures can never drift from what the adapter sends.

Every response is scrubbed by ALLOWLIST before it is saved: structural values
(`__typename`, `type`, `state`, flags, durations) are kept; node ids, database
ids, issue numbers, iteration ids and cursors are remapped to stable fakes; every
date is shifted by one whole-day offset; owners become `acme`, repositories
`platform` / `core`, logins `user<n>`, and every other string a neutral template.
The test `fixtures_are_scrubbed` re-checks every file structurally.

Each file is {"operation", "variables", "status", "body"} for GraphQL, or
{"request", "status", "body"} for REST.
"""

import argparse
import datetime as dt
import json
import os
import pathlib
import re
import sys
import urllib.error
import urllib.request

QUERIES = pathlib.Path(__file__).resolve().parent.parent / "crates/senseid/src/tracker/github_queries"
SHIFT_DAYS = -70
KEEP = {"__typename", "type", "state", "isArchived", "closed", "duration", "hasNextPage", "path"}


def document(name):
    doc = (QUERIES / f"{name}.graphql").read_text()
    if "...Board" in doc:
        doc += "\n" + (QUERIES / "board_fragment.graphql").read_text()
    return doc


def call(token, method, path, body=None):
    req = urllib.request.Request(
        f"https://api.github.com{path}",
        data=json.dumps(body).encode() if body is not None else None,
        method=method,
        headers={
            "Authorization": f"Bearer {token}",
            "User-Agent": "sensei-fixture-recorder",
            "Content-Type": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read() or b"null")


class Scrubber:
    """Consistent fakes for one fixture set."""

    def __init__(self, owner, repos):
        self.owner = owner.lower()
        self.repos = {r.lower(): fake for r, fake in repos.items()}
        self.maps = {}

    def fake(self, kind, real, fmt):
        m = self.maps.setdefault(kind, {})
        if real not in m:
            m[real] = fmt(len(m) + 1)
        return m[real]

    def node_id(self, v):
        for prefix in ("PVTIF_", "PVTI_", "PVT_"):
            if v.startswith(prefix):
                return self.fake(prefix, v, lambda n, p=prefix: f"{p}{n}")
        raise SystemExit(f"unexpected node id shape: {v[:6]}…")

    def repo(self, name_with_owner):
        owner, _, name = name_with_owner.partition("/")
        if owner.lower() != self.owner or name.lower() not in self.repos:
            raise SystemExit(f"repository outside the fixture: {name_with_owner}")
        return f"acme/{self.repos[name.lower()]}"

    def number(self, n):
        return self.fake("number", n, lambda k: k)

    def date(self, v):
        if re.fullmatch(r"\d{4}-\d{2}-\d{2}", v):
            return (dt.date.fromisoformat(v) + dt.timedelta(days=SHIFT_DAYS)).isoformat()
        t = dt.datetime.fromisoformat(v.replace("Z", "+00:00")) + dt.timedelta(days=SHIFT_DAYS)
        return t.strftime("%Y-%m-%dT%H:%M:%SZ")

    def walk(self, v, key=None, parent=None):
        if isinstance(v, dict):
            out = {}
            # an issue's url and repository need its (scrubbed) number and repo first
            ctx = dict(v)
            for k, x in v.items():
                out[k] = self.walk(x, k, ctx)
            return out
        if isinstance(v, list):
            return [self.walk(x, key, parent) for x in v]
        if v is None or isinstance(v, bool):
            return v
        if key in KEEP:
            return v
        if key == "id":
            if isinstance(v, str) and re.fullmatch(r"[0-9a-f]{8}", v):
                return self.fake("iteration", v, lambda n: f"it{n}")
            return self.node_id(v)
        if key in ("iterationId",):
            return self.fake("iteration", v, lambda n: f"it{n}")
        if key == "databaseId":
            is_user = parent is not None and "login" in parent
            return self.fake("user" if is_user else "issue", v, lambda n: (9000 if is_user else 1000) + n)
        if key == "number":
            # a project's number stays (it is the board address); an issue's is remapped
            return v if parent and "fields" in parent else self.number(v)
        if key in ("startDate", "createdAt", "updatedAt"):
            return self.date(v)
        if key == "endCursor":
            return self.fake("cursor", v, lambda n: f"c{n}")
        if key == "nameWithOwner":
            return self.repo(v)
        if key == "login":
            if v.lower() == self.owner:
                return "acme"
            return self.fake("login", v.lower(), lambda n: f"user{n}")
        if key == "url":
            return self.url(v)
        if key == "name":
            return "Sprint" if parent and "configuration" in parent else f"Type {self.fake('typename', v, lambda n: n)}"
        if key == "title":
            if parent and ("startDate" in parent or "iterationId" in parent):
                return f"Iteration {self.fake('ititle', v, lambda n: n)}"
            if parent and "fields" in parent:
                return f"Project {parent.get('number')}"
            return f"Issue {self.number(parent['number'])}" if parent and "number" in parent else "Title"
        if key == "body":
            return f"Body of issue {self.number(parent['number'])}" if parent and "number" in parent else "Body"
        if key == "message":
            return "scrubbed message"
        if isinstance(v, (int, float)):
            return v
        raise SystemExit(f"no scrub rule for {key!r}")

    def url(self, v):
        m = re.fullmatch(r"https://github\.com/orgs/[^/]+/projects/(\d+)", v)
        if m:
            return f"https://github.com/orgs/acme/projects/{m.group(1)}"
        m = re.fullmatch(r"https://github\.com/([^/]+/[^/]+)/issues/(\d+)", v)
        if m:
            return f"https://github.com/{self.repo(m.group(1))}/issues/{self.number(int(m.group(2)))}"
        m = re.fullmatch(r"https://github\.com/([^/]+/[^/]+)", v)
        if m:
            return f"https://github.com/{self.repo(m.group(1))}"
        raise SystemExit(f"unexpected url shape: {v[:30]}…")

    def variables(self, var):
        out = {}
        for k, x in var.items():
            if k == "owner":
                out[k] = "acme"
            elif k == "name":
                out[k] = self.repos[x.lower()]
            elif k == "number" and "name" in var:
                out[k] = self.number(x)
            elif k == "id":
                out[k] = self.node_id(x)
            elif k == "after":
                out[k] = None if x is None else self.fake("cursor", x, lambda n: f"c{n}")
            else:
                out[k] = x
        return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--owner", required=True)
    ap.add_argument("--project", type=int, required=True)
    ap.add_argument("--no-field-project", type=int, required=True)
    ap.add_argument("--repo", required=True, help="the repository of the --on/--off/--pr numbers")
    ap.add_argument("--other-repo", required=True, help="the second repository on the project")
    ap.add_argument("--field", default="Sprint")
    ap.add_argument("--on", type=int, required=True)
    ap.add_argument("--off", type=int, required=True)
    ap.add_argument("--pr", type=int, required=True)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    token = os.environ.get("GITHUB_TOKEN")
    if not token:
        sys.exit("GITHUB_TOKEN is not set")
    out = pathlib.Path(a.out)
    scrub = Scrubber(a.owner, {a.repo: "platform", a.other_repo: "core"})
    saved = []

    def gql(file, name, variables):
        status, body = call(token, "POST", "/graphql", {"query": document(name), "variables": variables})
        record = {
            "operation": re.search(r"query\s+(\w+)", document(name)).group(1),
            "variables": scrub.variables(variables),
            "status": status,
            "body": scrub.walk(body),
        }
        return file, record, body

    def save(dirname, file, record):
        p = out / dirname / f"{file}.json"
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")
        saved.append(str(p))

    # the project set
    file, rec, body = gql("validate_org", "validate_org", {"owner": a.owner, "number": a.project})
    save("project", file, rec)
    project_id = body["data"]["organization"]["projectV2"]["id"]
    file, rec, _ = gql("board", "board", {"id": project_id})
    save("project", file, rec)
    after, page = None, 1
    while True:
        file, rec, body = gql(f"items_p{page}", "items", {"id": project_id, "field": a.field, "after": after})
        save("project", file, rec)
        info = body["data"]["node"]["items"]["pageInfo"]
        if not info["hasNextPage"]:
            break
        after, page = info["endCursor"], page + 1
    status, user = call(token, "GET", "/user")
    save("project", "user", {"request": "/user", "status": status,
                              "body": {"login": "user1", "id": 9999, "type": user.get("type")}})

    # issues
    for name, repo, n in [("issue-on", a.repo, a.on), ("issue-off", a.repo, a.off),
                          ("pr", a.repo, a.pr), ("missing-repo", "does-not-exist-sensei-fixture", 1)]:
        if name == "missing-repo":
            scrub.repos[repo] = "missing"
        file, rec, _ = gql("issue", "issue", {"owner": a.owner, "name": repo, "number": n,
                                              "field": a.field, "after": None})
        save(name, file, rec)

    # no iteration field
    file, rec, body = gql("validate_org", "validate_org", {"owner": a.owner, "number": a.no_field_project})
    save("no-field", file, rec)
    nf_id = body["data"]["organization"]["projectV2"]["id"]
    file, rec, _ = gql("board", "board", {"id": nf_id})
    save("no-field", file, rec)
    file, rec, _ = gql("items_p1", "items", {"id": nf_id, "field": "", "after": None})
    save("no-field", file, rec)

    print("\n".join(saved))


if __name__ == "__main__":
    main()
