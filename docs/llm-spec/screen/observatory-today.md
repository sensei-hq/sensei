# 家 · Observatory · Today

**Segment:** 03 · Observatory — daily use
**Route:** `/` (root)
**Source mockup:** [`lib/observatory-today.jsx`](../../mockups/Sensei/lib/observatory-today.jsx) → `ObsHome` (default) with early/mature switch on `data.dataMaturity`
**App file:** `app/src/routes/(observatory)/+page.svelte`

## Purpose

The user walks in. They want to know **one** thing: what needs my
attention today, and can I trust it? Not five KPIs. Not a triage
queue. One focal *koan* — a compact teaching or observation that
Sensei is confident enough to surface — plus a short strip of
supporting insights, a lane showing what Sensei has adopted on their
behalf, a running FTR chart to say whether the pairing is trending up
or down, and a footer of recent sessions to jump into if they want to
verify anything.

The screen has **two mood states**. If the daemon has ≤ 4 enriched
sessions on the project, it renders the *early* variant: "Still
listening. A few early signals are forming. Nothing confident enough
to teach yet." If it has enough signal, it renders the *mature*
variant: a real koan with a specific action. **The early state is a
feature, not an empty state** — it explains what's happening while
Sensei is still calibrating.

Kanji is 家 — *home*.

## Data invariants

- `GET /api/observatory/today` returns
  ```json
  {
    "greeting": "Good morning",
    "today": "Wed · 22 Apr",
    "dataMaturity": "early" | "mature",
    "hero": { "kanji": "…", "koan": "…", "body": "…", "impact": "…", "action": "…" | null, "source": "…", "noticed": "…" },
    "insights": [ { "kanji": "…", "label": "…", "text": "…", "tag": "…", "tone": "warn|good|mute" }, … ],
    "adopted": [ { "when": "…", "what": "…", "scope": "…", "source": "…" }, … ],
    "recentSessions": [ { "id": "…", "when": "…", "project": "…", "ftr": "…", "summary": "…" }, … ]
  }
  ```
- FTR strip data comes from `GET /api/observatory/ftr` returning
  `{ ftr14d, ftr14dPrev, ftrTrend[], sessions7d }`.
- `dataMaturity === "early"` when the analyzer has enriched < 5
  sessions across the user's active projects. This is a daemon
  decision, not a UI heuristic — the screen must not decide.
- The koan on mature days points at real sessions
  (`source: "from s-2891 · s-2889 · s-2886"`). Any session id in
  `source` must resolve to a real `activity.sessions` row.

## Signals shown

| Element | Value | Meaning | Correct example |
|---|---|---|---|
| Greeting line | text | Time-of-day greeting + date | `Good morning · Wed · 22 Apr` |
| Hero kanji | 1 char | Domain kanji from the koan | `聴` |
| Koan title | 1 line | The teaching itself, ≤ 60 chars | `"The AI does not know your auth."` |
| Koan body | 2–3 sentences | Evidence for the teaching | `Three sessions corrected this week…` |
| Koan action | button label OR null | The next-step, when confident | `Draft a persona` (early: null) |
| Koan source | linked session ids | Provenance — clickable to /sessions | `from s-2891 · s-2889 · s-2886` |
| Insight card × ≤3 | kanji + label + text + tag + tone | Supporting observations behind the hero | `繰 · Pattern recurring · Cache invalidation missed again` |
| Adopted lane | list of ≤5 recent adoptions | What Sensei implemented on the user's behalf | `Canvas smoothing pattern → rule · lumen-studio` |
| FTR chip | 0–100 pct | 14d rolling first-turn resolution | `78%, up 6 pts` |
| FTR trend | 14-point sparkline | Prior 14 days | `0.71, 0.69, 0.74, … 0.78` |
| Recent sessions | ≤ 5 rows | Last handful of sessions with a chip per FTR | `s-2891 · lumen-auth · 15 min · ↑ FTR` |

## Done gate

- Loading `/` with the sensei binary on Jerry's data shows either the
  **early** or **mature** variant based on `dataMaturity`, not a mix.
- The koan renders one specific teaching, not a lorem-ipsum. In the
  early state the action button is absent, not a disabled shell.
- The FTR chip shows an integer + arrow (up / down / flat) reflecting
  `ftr14d - ftr14dPrev`.
- Every session id in `hero.source` resolves via
  `GET /api/sessions/{id}` (name-or-UUID resolution honored).
- Adopted lane shows real rows or is honestly empty
  ("Nothing adopted yet — Sensei is still watching.").
- Dark-mode: all four tones (warn/good/mute/plain) remain readable.
- One-decision-one-default theme is honoured: the action button on
  mature koan uses the standard Apply-family verb, not a bespoke
  label.

Optional check:
```
curl -s http://localhost:7744/api/observatory/today | jq '{maturity: .dataMaturity, has_hero: (.hero != null), insights_count: (.insights | length)}'
# early expected: {maturity: "early", has_hero: true, insights_count: 2}
# mature expected: {maturity: "mature", has_hero: true, insights_count: 3}
```

## Wrong gate

- **Both early and mature copy visible.** Maturity switch collapsed.
- **Koan is generic** ("Nothing to report today"). Should be an early
  state ("Still listening") or absent, never generic.
- **Insight tones don't match tokens.** Warn tone rendered on the
  success token, etc.
- **FTR trend flat at 0** while `ftr14d > 0`. Trend-data join broken.
- **Adopted lane populated with items that don't exist in
  `memories`.** Read-path bug — the list is showing something derived
  the wrong way.
- **Recent-sessions FTR badge shown but session detail says "no tool
  calls".** The client-session-id resolution regressed — same
  root cause we already fixed once.
- **Koan action button navigates nowhere** — the target route is not
  yet wired.

## Related

- [[pipeline/analyzer]] — where the maturity decision is made
- [[pipeline/ftr]] — the FTR chip
- [[pipeline/memory]] — what feeds the adopted lane
- [[pipeline/insights]] — the triage that produces the hero + insights
- [[screen/observatory-sessions]] — where the recent-sessions row jumps
