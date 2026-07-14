# 群 · Pipeline · Collective intelligence

**Owner files:**
- Federation transport: `crates/hive-mind/` + `crates/hive-protocol/`
- Anonymisation: `crates/senseid/src/collective/anonymize.rs`
- Community promotion: `crates/senseid/src/collective/promote.rs`
- Inbox: `crates/senseid/src/collective/inbox.rs`

**Companion design doc:** `docs/archive/ideas/29-telemetry.md`.

## Purpose

The best insight sensei can offer isn't from your own project —
it's *"other developers in the same stack keep hitting X; here's
what worked for them."* Collective intelligence is how those
cross-user, cross-project insights flow. The mechanism reuses
[[pipeline/dojo-lifecycle]] with `global-dojo` as the special-
case Dōjō everyone can join.

Three flows:

1. **Contribute** — the user's Sensei anonymises high-confidence
   patterns / memories / rules and ships them upstream to
   `global-dojo` (or opts out entirely; opt-in default is
   conservative).
2. **Promote** — the collective's maintainers (or an automated
   trust process — see below) triage contributions and publish
   the ones that clear the bar.
3. **Distribute** — approved community insights land downstream
   in every matching subscriber's Upgrades lane.

Community insights are inherently generalised — the user's own
project-specific facts stay local; only the underlying idea
travels.

Kanji is 群 — *collective*.

## Data invariants

- Modelled as a Dōjō membership with `kind: community` and
  `dojo_url: dojo.sensei-hq.org/org/global-dojo`. Uses the
  `dojo.*` schema.
- **Opt-in** — the user must explicitly enable "contribute to
  the collective" in Preferences → Sharing. Default off.
- Anonymisation is **stricter than a client-Dōjō dereference**:
  - Source references stripped.
  - Project names replaced with a project-shape descriptor
    (`{stack: [rust], size: medium, kind: web-service}`) so the
    receiving side can filter for relevance without seeing the
    name.
  - User identity replaced with a stable anonymous id per
    user (rotated periodically).
  - Any code snippets rewritten stack-agnostic via
    [[pipeline/inferencing]] `reasoning` chain; snippets
    that resist generalisation are dropped.

## Contribution criteria

Not every memory / pattern / rule contributes. Bars:

- **Memory** — `state = battle-tested` AND `strength ≥ 0.7` AND
  `scope = user` or wider (project-scope memories require the
  user to widen first).
- **Pattern** — codebase or library pattern with
  `ftr_delta_observed ≥ +0.05` on at least 2 users' data (once
  cross-user data exists — see Bootstrap below).
- **Rule** — promoted from a memory that meets the memory
  criteria.

## Community promotion (triage on the receiving side)

`global-dojo` runs an automated first-pass triage plus a
human-in-the-loop layer:

1. **Cluster** — group similar incoming contributions by
   signature.
2. **Score** — count of contributing users; average confidence;
   cross-project applicability.
3. **Auto-approve** — items that clear a high bar (e.g.
   contributed by ≥ 5 users with `confidence ≥ 0.8`) publish
   automatically.
4. **Human triage** — everything else waits for a maintainer to
   review. Maintainers are trusted community members with a
   named identity.
5. **Publish** — approved items land in the community catalogue,
   attributed to the maintainer + the "N contributors from …
   projects" summary.

## Bootstrap problem

There's no cross-user data until people contribute. Sensei ships
with a **seed catalogue** — hand-curated community insights from
the sensei team plus early partners. This bootstraps the
downstream lane so a new user sees something meaningful before
their own contributions accrue.

## Signals produced

| Signal | Consumer |
|---|---|
| Contribution queue (upstream) | Share-review batch UI |
| Community catalogue (downstream) | Upgrades lane |
| Attribution string per landing artifact | Upgrades card ("from 12 rust users") |
| Community-vs-personal comparisons | Insights ("this project's FTR is 0.6; peers at 0.7") |

## Done gate

- Enabling collective sharing in Preferences begins contributing
  eligible artifacts on the next batch tick.
- Anonymisation strips source references AND replaces user
  identity AND generalises code snippets before contributions
  leave the machine.
- Downstream lane in Upgrades populates from the seed catalogue
  on install; community-derived items land as they clear
  triage.
- Community-vs-personal comparison metrics render on the Impact
  screen (or the Today screen) once the user's project has 30d
  of data.
- Disabling collective sharing stops future contributions
  immediately; already-contributed items stay published (they
  can't be recalled from a distributed system, but user is told
  that up-front).

## Wrong gate

- **A contribution ships with a project name intact.**
  Anonymisation strip failed — hard confidentiality regression.
- **Downstream lane empty on install.** Seed catalogue missing
  or not indexed.
- **Community insight lands in a project scope that doesn't
  match the artifact's declared scope.** Distribution filter
  wrong.
- **Contribution counter says N users but the receiving side
  can identify individuals.** Anonymisation isn't truly
  irreversible — needs a k-anonymity check on the receiving
  side.
- **Auto-approval publishes low-confidence content.** Bar
  wrong.
- **Attribution says "from anonymous" on artifacts the user
  contributed themselves.** UI should show "you contributed to
  this" on landing artifacts the user was one of the sources
  for (privacy-preserving cross-check).

## Related

- [[pipeline/dojo-lifecycle]] — the mechanism this reuses via
  `global-dojo`
- [[pipeline/memory]] — memory promotion source
- [[pipeline/patterns]] — pattern promotion source
- [[pipeline/governance]] — rule promotion source
- [[pipeline/inferencing]] — code-snippet generalisation
- [[screen/observatory-collective]] — user-facing controls
- [[screen/observatory-upgrades]] — downstream lane
- (archive: ideas/29-telemetry.md) — source design
