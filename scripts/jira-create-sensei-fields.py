#!/usr/bin/env python3
"""Create the Jira number fields sensei writes usage into, and map them in sensei.

Write-back (spec/daemon/tracker-metrics-writeback.md W6, W7) writes a board's usage
into custom fields that already exist; sensei never creates them, because creating a
field and putting it on screens needs Jira admin rights a person's token rarely has.
A Jira admin runs this once per board instead:

  1. For each metric, a Number custom field named "sensei <label>" is found or created
     (a found field must already be a Number field), with a global context.
  2. The field is added to the edit and view screens of every project on the board, so
     the REST write is accepted (a field missing from the edit screen is a 400, which
     sensei records as `field_invalid`). Team-managed projects have no shared screens:
     they are reported and skipped.
  3. With --sensei, the board's mapping in the local daemon is saved through
     `PUT /api/workspaces/{id}/trackers/{tracker}/fields`, so nobody types the ids; the
     daemon still runs its own checks. A metric this run doesn't map keeps its row.

The metrics and their labels are read from the repo (the `tracker_metric` enum and the
app's `METRICS` list), so a new metric needs no edit here.

Usage:
    JIRA_API_TOKEN=… jira-create-sensei-fields.py --site https://acme.atlassian.net \\
        --email admin@acme.com --board 42 [--metrics sessions,api_cost] [--sensei] [--dry-run]
    jira-create-sensei-fields.py --self-test

The token is read from $JIRA_API_TOKEN or prompted for, never taken as an argument.
--dry-run makes only GET requests and says what it would do.
"""
from __future__ import annotations

import argparse
import base64
import getpass
import json
import os
import pathlib
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

REPO = pathlib.Path(__file__).resolve().parent.parent
ENUM_DDL = REPO / "database/ddl/enum/sensei/tracker_metric.ddl"
APP_METRICS = REPO / "app/src/routes/(workspace)/workspace/[id]/integrations/integrations-state.svelte.ts"
NUMBER_TYPE = "com.atlassian.jira.plugin.system.customfieldtypes:float"
NUMBER_SEARCHER = "com.atlassian.jira.plugin.system.customfieldtypes:exactnumber"
DESCRIPTION = "Usage written by sensei after each sync (tracker metrics write-back). Edits are overwritten."
SENSEI_URL = "http://127.0.0.1:7744"


class Failure(Exception):
    """A stop with a message for the admin; nothing after it ran."""


def load_metrics(ddl: str, app: str) -> list[tuple[str, str]]:
    """The metrics in enum order with the app's labels; the two sources must agree."""
    body = re.search(r"create type tracker_metric as enum \((.*?)\);", ddl, re.S)
    if not body:
        raise Failure(f"no tracker_metric enum in {ENUM_DDL}")
    names = re.findall(r"'([a-z_]+)'", body.group(1))
    labels = dict(re.findall(r"\{ metric: '([a-z_]+)', label: '([^']+)' \}", app))
    if set(labels) != set(names):
        raise Failure(f"the enum and the app's METRICS disagree: {sorted(set(names) ^ set(labels))}")
    return [(n, labels[n]) for n in names]


def field_name(label: str) -> str:
    return f"sensei {label}"


class Http:
    """JSON over HTTP; `send` is the one seam a test replaces."""

    def __init__(self, base: str, headers: dict[str, str]):
        self.base, self.headers = base.rstrip("/"), headers

    def send(self, method: str, path: str, body: object | None) -> object:
        data = None if body is None else json.dumps(body).encode()
        req = urllib.request.Request(self.base + path, data=data, method=method)
        for k, v in {**self.headers, "Accept": "application/json", "Content-Type": "application/json"}.items():
            req.add_header(k, v)
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                raw = r.read()
        except urllib.error.HTTPError as e:
            detail = e.read().decode(errors="replace")[:300]
            hint = " (does this account have Jira admin rights?)" if e.code in (401, 403) else ""
            raise Failure(f"{method} {path} → {e.code}{hint}: {detail}") from None
        except urllib.error.URLError as e:
            raise Failure(f"{method} {path}: {e.reason}") from None
        return json.loads(raw) if raw else None

    def get(self, path: str) -> object:
        return self.send("GET", path, None)

    def pages(self, path: str) -> list[dict]:
        """Every `values` item of a paged Jira list."""
        out: list[dict] = []
        start = 0
        while True:
            sep = "&" if "?" in path else "?"
            page = self.get(f"{path}{sep}startAt={start}")
            values = page.get("values", [])
            out += values
            if page.get("isLast", True) or not values:
                return out
            start += len(values)


class Run:
    def __init__(self, jira: Http, dry_run: bool, say=print):
        self.jira, self.dry_run, self.say = jira, dry_run, say

    def write(self, method: str, path: str, body: object, what: str) -> object | None:
        if self.dry_run:
            self.say(f"  would {what}")
            return None
        self.say(f"  {what}")
        return self.jira.send(method, path, body)

    def fields(self, metrics: list[tuple[str, str]]) -> dict[str, str]:
        """metric → field id, found by name or created; a dry run leaves new ones unnamed."""
        existing = self.jira.get("/rest/api/3/field")
        by_name: dict[str, list[dict]] = {}
        for f in existing:
            by_name.setdefault(f.get("name"), []).append(f)
        out: dict[str, str] = {}
        for metric, label in metrics:
            name = field_name(label)
            found = by_name.get(name, [])
            if len(found) > 1:
                raise Failure(f'{len(found)} fields are named "{name}"; rename the extras in Jira and run again')
            if found:
                f = found[0]
                if f.get("schema", {}).get("custom") != NUMBER_TYPE:
                    raise Failure(f'"{name}" ({f["id"]}) exists but isn\'t a Number field')
                self.say(f'  "{name}": {f["id"]} (exists)')
                out[metric] = f["id"]
                continue
            made = self.write(
                "POST",
                "/rest/api/3/field",
                {"name": name, "description": DESCRIPTION, "type": NUMBER_TYPE, "searcherKey": NUMBER_SEARCHER},
                f'create the Number field "{name}"',
            )
            if made is not None:
                out[metric] = made["id"]
                self.say(f'  "{name}": {made["id"]} (created)')
                self.context(made["id"])
        return out

    def context(self, field_id: str) -> None:
        """A field with no context can't hold a value anywhere: give it a global one."""
        if self.jira.pages(f"/rest/api/3/field/{field_id}/context"):
            return
        self.write(
            "POST",
            f"/rest/api/3/field/{field_id}/context",
            {"name": "Default", "projectIds": [], "issueTypeIds": []},
            f"give {field_id} a global context",
        )

    def screens(self, board: str) -> list[str]:
        """The edit and view screens of the board's company-managed projects."""
        screens: set[str] = set()
        for p in self.jira.pages(f"/rest/agile/1.0/board/{urllib.parse.quote(board)}/project"):
            project = self.jira.get(f"/rest/api/3/project/{p['id']}")
            if project.get("style") == "next-gen":
                self.say(f"  {p['key']} is team-managed: add the fields in its own settings (skipped)")
                continue
            schemes = self.jira.pages(f"/rest/api/3/issuetypescreenscheme/project?projectId={p['id']}")
            if not schemes:
                raise Failure(f"{p['key']} has no issue type screen scheme")
            for s in schemes:
                scheme_id = s["issueTypeScreenScheme"]["id"]
                mapping = self.jira.pages(f"/rest/api/3/issuetypescreenscheme/mapping?issueTypeScreenSchemeId={scheme_id}")
                ids = sorted({m["screenSchemeId"] for m in mapping})
                if not ids:
                    continue
                query = "&".join(f"id={i}" for i in ids)
                for ss in self.jira.pages(f"/rest/api/3/screenscheme?{query}"):
                    sc = ss.get("screens", {})
                    for kind in ("edit", "view"):
                        screen = sc.get(kind, sc.get("default"))
                        if screen is not None:
                            screens.add(str(screen))
        return sorted(screens, key=int)

    def place(self, screen: str, field_ids: list[str]) -> None:
        """Add each field to the screen's first tab unless a tab already has it."""
        tabs = self.jira.get(f"/rest/api/3/screens/{screen}/tabs")
        if not tabs:
            raise Failure(f"screen {screen} has no tabs")
        on = set()
        for t in tabs:
            on |= {f["id"] for f in self.jira.get(f"/rest/api/3/screens/{screen}/tabs/{t['id']}/fields")}
        for fid in field_ids:
            if fid not in on:
                self.write(
                    "POST",
                    f"/rest/api/3/screens/{screen}/tabs/{tabs[0]['id']}/fields",
                    {"fieldId": fid},
                    f"add {fid} to screen {screen}",
                )


def sensei_tracker(sensei: Http, site: str, board: str) -> tuple[str, str]:
    """(workspace id, tracker id) of the board in the local daemon; exactly one must match."""
    site = site.rstrip("/").lower()
    hits = []
    for w in sensei.get("/api/workspaces"):
        for t in sensei.get(f"/api/workspaces/{w['id']}/trackers"):
            if str(t.get("base_url", "")).rstrip("/").lower() == site and str(t.get("board_ref")) == board:
                hits.append((w["id"], t["id"]))
    if len(hits) != 1:
        raise Failure(f"{len(hits)} sensei boards match {site} board {board}; connect it in sensei first" if not hits
                      else f"{len(hits)} sensei boards match {site} board {board}; map it in the Fields editor")
    return hits[0]


def merged(current: list[dict], mapped: dict[str, str]) -> list[dict]:
    """The board's mapping with this run's metrics replaced and every other row kept."""
    ours = set(mapped.values())
    kept = [r for r in current if r["metric"] not in mapped and r["field_ref"] not in ours]
    return kept + [{"metric": m, "field_ref": f} for m, f in mapped.items()]


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--site", required=True, help="https://<site>.atlassian.net")
    ap.add_argument("--email", required=True, help="the Jira admin's account email")
    ap.add_argument("--board", required=True, help="the board id, as in the board URL")
    ap.add_argument("--metrics", help="comma-separated metrics (default: all)")
    ap.add_argument("--sensei", action="store_true", help="also save the mapping in the local sensei daemon")
    ap.add_argument("--sensei-url", default=SENSEI_URL)
    ap.add_argument("--dry-run", action="store_true")
    a = ap.parse_args(argv)
    try:
        metrics = load_metrics(ENUM_DDL.read_text(), APP_METRICS.read_text())
        if a.metrics:
            wanted = [m.strip() for m in a.metrics.split(",") if m.strip()]
            unknown = [m for m in wanted if m not in dict(metrics)]
            if unknown:
                raise Failure(f"unknown metrics {unknown}; known: {', '.join(m for m, _ in metrics)}")
            metrics = [(m, l) for m, l in metrics if m in wanted]
        token = os.environ.get("JIRA_API_TOKEN") or getpass.getpass("Jira API token: ")
        auth = base64.b64encode(f"{a.email}:{token}".encode()).decode()
        run = Run(Http(a.site, {"Authorization": f"Basic {auth}"}), a.dry_run)
        print("Fields:")
        mapped = run.fields(metrics)
        print("Screens:")
        for screen in run.screens(a.board):
            run.place(screen, list(mapped.values()))
        if a.sensei:
            sensei = Http(a.sensei_url, {})
            wid, tid = sensei_tracker(sensei, a.site, a.board)
            path = f"/api/workspaces/{wid}/trackers/{tid}/fields"
            body = {"fields": merged(sensei.get(path)["fields"], mapped)}
            if a.dry_run:
                print(f"sensei: would save {json.dumps(body)}")
            else:
                sensei.send("PUT", path, body)
                print(f"sensei: mapped {len(mapped)} fields on this board")
        else:
            print("Mapping (Fields editor, or rerun with --sensei):")
            for m, f in mapped.items():
                print(f"  {m} → {f}")
    except Failure as e:
        print(f"jira-create-sensei-fields: {e}", file=sys.stderr)
        return 1
    return 0


# ---- self-test -------------------------------------------------------------


class FakeJira(Http):
    """An in-memory Jira: fields, contexts, one board over a classic and a team-managed project."""

    def __init__(self, fields: list[dict] | None = None):
        super().__init__("https://fake", {})
        self.fields = list(fields or [])
        self.contexts: dict[str, list] = {}
        self.tabs = {"10": {"100": []}, "11": {"110": [], "111": []}, "12": {"120": []}}
        self.sent: list[tuple[str, str]] = []

    def send(self, method, path, body):
        self.sent.append((method, path))
        p = urllib.parse.urlparse(path)
        q = urllib.parse.parse_qs(p.query)
        page = lambda vals: {"values": vals, "isLast": True}
        if (method, p.path) == ("GET", "/rest/api/3/field"):
            return self.fields
        if (method, p.path) == ("POST", "/rest/api/3/field"):
            f = {"id": f"customfield_{10100 + len(self.fields)}", "name": body["name"],
                 "schema": {"custom": body["type"]}}
            self.fields.append(f)
            return f
        if m := re.fullmatch(r"/rest/api/3/field/(\w+)/context", p.path):
            if method == "POST":
                self.contexts.setdefault(m.group(1), []).append(body)
                return {"id": "1"}
            return page(self.contexts.get(m.group(1), []))
        if p.path == "/rest/agile/1.0/board/42/project":
            return page([{"id": "1", "key": "AUTH"}, {"id": "2", "key": "TEAM"}])
        if p.path == "/rest/api/3/project/1":
            return {"style": "classic"}
        if p.path == "/rest/api/3/project/2":
            return {"style": "next-gen"}
        if p.path == "/rest/api/3/issuetypescreenscheme/project":
            return page([{"issueTypeScreenScheme": {"id": "5"}}] if q["projectId"] == ["1"] else [])
        if p.path == "/rest/api/3/issuetypescreenscheme/mapping":
            return page([{"screenSchemeId": "7"}, {"screenSchemeId": "8"}, {"screenSchemeId": "7"}])
        if p.path == "/rest/api/3/screenscheme":
            assert q["id"] == ["7", "8"], q
            return page([{"screens": {"default": 10, "edit": 11}}, {"screens": {"default": 12}}])
        if m := re.fullmatch(r"/rest/api/3/screens/(\d+)/tabs", p.path):
            return [{"id": t} for t in self.tabs[m.group(1)]]
        if m := re.fullmatch(r"/rest/api/3/screens/(\d+)/tabs/(\d+)/fields", p.path):
            tab = self.tabs[m.group(1)][m.group(2)]
            if method == "POST":
                tab.append({"id": body["fieldId"]})
                return {"id": body["fieldId"]}
            return tab
        raise AssertionError(f"unexpected {method} {path}")


def self_test() -> int:
    fails: list[str] = []

    def check(ok: bool, what: str) -> None:
        if not ok:
            fails.append(what)

    metrics = load_metrics(ENUM_DDL.read_text(), APP_METRICS.read_text())
    check(metrics[0] == ("sessions", "Sessions") and len(metrics) == 13, "the repo's 13 metrics load in enum order")
    try:
        load_metrics("create type tracker_metric as enum ('a', 'b');", "{ metric: 'a', label: 'A' }")
        check(False, "an enum/app disagreement stops")
    except Failure:
        pass

    two = [("sessions", "Sessions"), ("api_cost", "API cost (USD)")]
    j = FakeJira([{"id": "customfield_10050", "name": "sensei Sessions", "schema": {"custom": NUMBER_TYPE}}])
    quiet = Run(j, dry_run=False, say=lambda _: None)
    mapped = quiet.fields(two)
    check(mapped["sessions"] == "customfield_10050", "an existing Number field is reused")
    check(mapped["api_cost"].startswith("customfield_") and mapped["api_cost"] != "customfield_10050", "a missing field is created")
    check(j.contexts.get(mapped["api_cost"]) == [{"name": "Default", "projectIds": [], "issueTypeIds": []}],
          "a created field gets a global context")
    screens = quiet.screens("42")
    check(screens == ["10", "11", "12"], f"edit and view screens of the classic project only, deduplicated: {screens}")
    j.tabs["11"]["111"].append({"id": "customfield_10050"})
    for s in screens:
        quiet.place(s, list(mapped.values()))
    check(j.tabs["11"]["110"] == [{"id": mapped["api_cost"]}], "a field already on a later tab isn't added again")
    check(all(len(j.tabs[s][next(iter(j.tabs[s]))]) == 2 for s in ("10", "12")), "both fields land on each first tab")
    before = len([s for s in j.sent if s[0] == "POST"])
    for s in screens:
        quiet.place(s, list(mapped.values()))
    quiet.fields(two)
    check(len([s for s in j.sent if s[0] == "POST"]) == before, "a second run changes nothing")

    wrong = FakeJira([{"id": "customfield_1", "name": "sensei Sessions", "schema": {"custom": "textfield"}}])
    try:
        Run(wrong, False, say=lambda _: None).fields(two)
        check(False, "a same-named field of another type stops the run")
    except Failure:
        pass

    dry = FakeJira()
    said: list[str] = []
    Run(dry, dry_run=True, say=said.append).fields(two)
    check(all(m == "GET" for m, _ in dry.sent), "a dry run only reads")
    check(any("would create" in s for s in said), "a dry run says what it would do")

    current = [{"metric": "sessions", "field_ref": "customfield_1"}, {"metric": "tokens_out", "field_ref": "customfield_2"}]
    check(merged(current, {"sessions": "customfield_9"}) ==
          [{"metric": "tokens_out", "field_ref": "customfield_2"}, {"metric": "sessions", "field_ref": "customfield_9"}],
          "the sensei mapping keeps other metrics and replaces this run's")

    class FakeSensei(Http):
        def __init__(self, trackers):
            super().__init__("http://x", {})
            self.trackers = trackers

        def send(self, method, path, body):
            if path == "/api/workspaces":
                return [{"id": "w1"}, {"id": "w2"}]
            return self.trackers.get(path.split("/")[3], [])

    s = FakeSensei({"w2": [{"id": "t1", "base_url": "https://Acme.atlassian.net/", "board_ref": "42"}]})
    check(sensei_tracker(s, "https://acme.atlassian.net", "42") == ("w2", "t1"), "the board is found across workspaces")
    try:
        sensei_tracker(s, "https://acme.atlassian.net", "43")
        check(False, "an unconnected board stops")
    except Failure:
        pass

    for f in fails:
        print(f"FAIL: {f}", file=sys.stderr)
    if not fails:
        print("jira-create-sensei-fields: self-test passed")
    return 1 if fails else 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        sys.exit(self_test())
    sys.exit(main(sys.argv[1:]))
