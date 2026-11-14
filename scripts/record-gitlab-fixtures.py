#!/usr/bin/env python3
"""Record the GitLab API responses the tracker adapter's tests run on (spec
docs/spec/daemon/tracker-gitlab.md, build step 1). Real responses, never
hand-written: the spec's Wrong gate.

    GITLAB_TOKEN=glpat-... scripts/record-gitlab-fixtures.py \\
        --base https://gitlab.example.com --group my-group --project my-group/backend \\
        --out crates/senseid/tests/fixtures/gitlab/ce-no-milestones

The token is read from the environment only and never written. Every response is
scrubbed before it is saved: titles and descriptions, names, usernames, emails,
avatars and label names are replaced; the host becomes gitlab.example.com and the
group path becomes `acme`. Structure, ids, states, dates and types are kept,
because they are what the adapter reads.

Each file is {"request", "status", "headers": {x-next-page, x-total}, "body"}.
"""

import argparse
import json
import os
import pathlib
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

KEEP_HEADERS = ("x-next-page", "x-total", "x-total-pages", "x-page", "x-per-page")
# Free text and identity fields replaced wholesale; everything else is structural.
TEXT_FIELDS = {"description", "description_html", "title_html", "name_with_namespace"}
PERSON_FIELDS = {"name", "username", "email", "public_email", "avatar_url", "web_url"}


def get(base, token, path):
    req = urllib.request.Request(f"{base}/api/v4/{path}", headers={"PRIVATE-TOKEN": token})
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            status, headers, raw = r.status, r.headers, r.read()
    except urllib.error.HTTPError as e:
        status, headers, raw = e.code, e.headers, e.read()
    try:
        body = json.loads(raw) if raw else None
    except json.JSONDecodeError:
        body = raw.decode(errors="replace")[:200]
    kept = {h: headers.get(h) for h in KEEP_HEADERS if headers.get(h) is not None}
    return status, kept, body


class Scrubber:
    def __init__(self, host, group):
        self.host, self.group = host, group
        self.people = {}

    def person(self, obj):
        key = obj.get("id")
        n = self.people.setdefault(key, len(self.people) + 1)
        for f in PERSON_FIELDS & obj.keys():
            obj[f] = {
                "name": f"Person {n}",
                "username": f"person{n}",
                "web_url": f"https://gitlab.example.com/person{n}",
            }.get(f)
        return obj

    def text(self, s):
        s = s.replace(self.host, "gitlab.example.com")
        # Any other host under the same domain (a registry, pages): example.com.
        domain = ".".join(self.host.split(".")[-2:])
        s = re.sub(rf"[A-Za-z0-9.-]*{re.escape(domain)}", "example.com", s)
        # A group or project bot's generated name.
        s = re.sub(r"(group|project)_\d+_bot_[0-9a-f]+", r"\1_bot", s)
        # Any email address: its local part can name a person or a group.
        s = re.sub(r"[A-Za-z0-9._%+-]+@([A-Za-z0-9.-]+\.[A-Za-z]{2,})", r"someone@\1", s)
        # The group, as a whole path segment or display name, in any case.
        return re.sub(
            rf"(?<![A-Za-z0-9_.\-]){re.escape(self.group)}(?![A-Za-z0-9_.\-])", "acme", s, flags=re.I
        )

    def walk(self, v, parent_key=None):
        if isinstance(v, dict):
            if parent_key in {"author", "assignee", "closed_by", "merged_by"} or (
                parent_key == "assignees"
            ):
                self.person(v)
            out = {}
            for k, x in v.items():
                if k in TEXT_FIELDS:
                    out[k] = None
                elif k == "title" and "iid" in v:
                    kind = "Milestone" if "due_date" in v and "issue_type" not in v else "Issue"
                    out[k] = f"{kind} {v['iid']}"
                elif k == "labels" and isinstance(x, list):
                    out[k] = [f"label-{i + 1}" for i in range(len(x))]
                elif k == "username" and isinstance(x, str) and x.startswith("group_"):
                    out[k] = "group_bot"
                else:
                    out[k] = self.walk(x, k)
            return out
        if isinstance(v, list):
            return [self.walk(x, parent_key) for x in v]
        if isinstance(v, str):
            return self.text(v)
        return v


def enc(path):
    return urllib.parse.quote(path, safe="")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", required=True)
    ap.add_argument("--group", required=True, help="the group's full path")
    ap.add_argument("--project", help="one project's full path, for a project board")
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    token = os.environ.get("GITLAB_TOKEN")
    if not token:
        sys.exit("GITLAB_TOKEN is not set")
    base = a.base.rstrip("/")
    host = urllib.parse.urlparse(base).hostname
    scrub = Scrubber(host, a.group)
    out = pathlib.Path(a.out)
    out.mkdir(parents=True, exist_ok=True)

    def record(name, path):
        status, headers, body = get(base, token, path)
        doc = {
            "request": scrub.text(path.replace(enc(a.group), enc("acme")).replace(a.group, "acme")),
            "status": status,
            "headers": headers,
            "body": scrub.walk(body),
        }
        (out / f"{name}.json").write_text(json.dumps(doc, indent=2, sort_keys=True) + "\n")
        n = len(body) if isinstance(body, list) else ""
        print(f"{name:<28} {status} {n}")
        return status, body

    record("user", "user")
    record("version", "version")
    # Board resolution (§G1): a group path is a 404 as a project, then a group.
    record("project_by_group_path", f"projects/{enc(a.group)}")
    _, group = record("group", f"groups/{enc(a.group)}")
    gid = group["id"]
    record("group_iterations_probe", f"groups/{gid}/iterations?per_page=1")
    record(
        "group_milestones",
        f"groups/{gid}/milestones?state=all&include_ancestors=true&include_descendants=true&per_page=100",
    )
    page = 1
    while True:
        status, body = record(
            f"group_issues_p{page}",
            f"groups/{gid}/issues?scope=all&state=all&per_page=100&page={page}",
        )
        if status != 200 or len(body) < 100:
            break
        page += 1
    if a.project:
        _, proj = record("project", f"projects/{enc(a.project)}")
        pid = proj["id"]
        record("project_milestones", f"projects/{pid}/milestones?state=all&include_ancestors=true&per_page=100")
        record("project_issues_p1", f"projects/{pid}/issues?state=all&per_page=100&page=1")
        issues = record("_tmp", f"projects/{pid}/issues?state=all&per_page=1")[1]
        (out / "_tmp.json").unlink()
        if issues:
            iid = issues[0]["iid"]
            record("project_issue_one", f"projects/{pid}/issues/{iid}")
        record("project_issue_missing", f"projects/{pid}/issues/999999")


if __name__ == "__main__":
    main()
