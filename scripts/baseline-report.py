#!/usr/bin/env python3
"""Turn a baseline crawl (JSON lines from baseline-crawl-inspect.spec.ts) into a
markdown report: per screen, whether it rendered, its console errors, its error
states, and every click that errored, hit "Not found", or left a blank page."""
import json, re, sys
from collections import Counter

def load(paths):
    rows = {}
    for p in paths:
        for line in open(p):
            if line.strip():
                r = json.loads(line); rows[r['route']] = r
    return [rows[k] for k in sorted(rows)]

def lands_on(r):
    """Whether the screen stayed on its route, or the app sent it elsewhere."""
    want = r.get('resolved') or r['route']
    pattern = '^' + re.sub(r'\\\[[a-z]+\\\]', '[^/]+', re.escape(want)) + '(/|$)'
    return re.match(pattern, r['page']['path'].split('?')[0]) is not None

# A banner every screen shows (the dōjō sync failing) is one finding, not one
# per screen; a "Dismiss" button caught by the error-state selector is no error,
# and neither is a short label drawn in the danger colour.
GLOBAL = re.compile(r'personas failed|keychain access needs approval')
NOT_AN_ERROR = re.compile(r'^Dismiss$')
seen_global = set()

def states(found):
    out = []
    for e in found:
        if NOT_AN_ERROR.match(e.strip()): continue
        # A legend or a badge in the danger colour ("Failure", "high", "0 broken")
        # is a label; an error says what went wrong, in a sentence.
        if len(e.split()) < 3: continue
        if GLOBAL.search(e): seen_global.add(e[:160]); continue
        out.append(e)
    return out

def issues(r):
    """Each issue as (kind, text): the kind counts, the text says where."""
    out = []
    pg = r['page']
    if not lands_on(r):
        # The clicks below belong to the screen it landed on, not this one.
        return [('redirect', f"redirects to `{pg['path']}`")]
    if pg['text'] < 40: out.append(('blank', f"renders almost nothing ({pg['text']} chars)"))
    if pg['notFound']: out.append(('not-found', 'shows "Not found"'))
    for e in states(pg['errorStates']): out.append(('error-state', f"error state: {e[:120]}"))
    for e in r['errors']: out.append(('console', f"console {e['kind']}: {e['text'][:140]}"))
    for c in r['clicks']:
        where = f"click `{c['label']}`"
        if not c['found']:
            out.append(('click-missed', f"{where}: element gone before it could be clicked")); continue
        a = c['after']
        if a['notFound']: out.append(('click-not-found', f"{where} → {a['path']}: Not found"))
        elif a['text'] < 40: out.append(('click-blank', f"{where} → {a['path']}: blank ({a['text']} chars)"))
        for e in states(a['errorStates'])[:2]: out.append(('click-error-state', f"{where} → error state: {e[:100]}"))
        for e in c['errors'][:3]: out.append(('click-console', f"{where} → console {e['kind']}: {e['text'][:120]}"))
    return out

def main():
    rows = load([a for a in sys.argv[1:] if not a.startswith('--froze=')])
    froze = [a.split('=', 1)[1] for a in sys.argv[1:] if a.startswith('--froze=')]
    print('| Screen | Clickables | Clicked | Issues |\n|---|---|---|---|')
    allissues = []
    for r in rows:
        iss = issues(r); allissues += [(r['route'], i) for i in iss]
        print(f"| `{r['route']}` | {r['clickables']} | {len(r['clicks'])} | {len(iss)} |")
    for f in froze:
        print(f"| `{f}` | — | — | froze the page |")
        allissues.append((f, ('froze', 'the page stopped answering (a 30s script call timed out) — nothing could be clicked')))
    print('\n## Issues by screen\n')
    cur = None
    for route, (_, i) in allissues:
        if route != cur: print(f"\n### `{route}`\n"); cur = route
        print(f"- {i}")
    if seen_global:
        print('\n## On every screen\n')
        for g in sorted(seen_global): print(f"- banner: {g}")
    kinds = Counter(k for _, (k, _) in allissues)
    print('\n## Totals\n'); [print(f"- {k}: {v}") for k, v in kinds.most_common()]

main()
