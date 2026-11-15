# Website · P2

**Plane:** platform · **Status:** [roadmap/05-platform.md](../../../../roadmap/05-platform.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
client / workspace stakeholder ([persona 7](../../../03-personas.md#7--client--workspace-stakeholder--the-narrow-lens))

## Purpose

The website is the **marketing site as a P2 module** — it must **only claim what
is built**; reorg §12 tracks the claims that already slipped (D-11). The
site is a SvelteKit app in the sibling repository
[sensei-hq/website](https://github.com/sensei-hq/website) (#346) that sells the built product, not the
roadmap. The job is **every claim traces to a roadmap status**; the site is
written *after* the roadmap so its claims trace to evidence. The user feels
this as *what the site promises is what the product does.*

> **Status honesty.** The SvelteKit marketing site exists in the sibling repository
> `sensei-hq/website` (#346), not in this tree. The
> GitHub Sponsors link is live. **One claim was already dropped** (D-11). The site's
> own dependency drift closed with the rest (`eefc6c119`). The site must be audited
> against the roadmap before any deploy.

## In scope / out of scope

- In: the marketing site pages (landing, features, pricing, docs, blog,
  changelog); the install command (`curl ... | sh`); the GitHub Sponsors link;
  **claim audit** — every feature claim on the site maps to a roadmap status
  (Built / Partial / Mock-only / Designed / Dormant / Gap).
- Out: the install implementation (P1), the CI/harness (P3), the Dōjō SaaS
  console (T2), the Dōjō web app (`dojo/`).

## Surfaces

- **App** — in [sensei-hq/website](https://github.com/sensei-hq/website): `src/routes/*`
  (SvelteKit pages); `src/lib/components/` (shared components); `static/` (assets).
- **Data** — roadmap statuses (sourced from `roadmap/*.md` at build time or via
  a content layer); changelog from git tags.
- **CI** — the claim audit runs in CI (planned) — any claim without a roadmap
  trace fails the build.

## Data invariants

- **Lead.** **Every claim traces to the roadmap.** If a page says "X is
  built", `roadmap/03-delivery.md` (or the relevant plane) must show ✅ for X.
- **Lead.** **No roadmap language in marketing.** The site says "sensei does X"
  — not "sensei will do X" or "sensei aims to do X".
- **Lead.** **Pricing page reflects the final model.** Free = public repositories
  and every personal dōjō (private repositories included), forever; Paid = an
  organisation's private repositories, on its admin's or owner's subscription, per
  active contributor; a solo developer gets everything free, blocking targets included
  (D52); local sensei is never limited by plan (D46); no Enterprise tier;
  GitHub Sponsors optional.
- **Lead.** **Install command is the unified one-liner** (P1) — not
  platform-specific instructions.
- **Lead.** **Changelog is generated from git tags** — not hand-written.

## Signals shown

| Element | Meaning | Worked example | Decision it drives / next step |
|---|---|---|---|
| Feature badge | Built / Partial / Mock-only / Designed | `Observatory · Built` | try what is Built; don't plan on Designed (the claim audit fails a badge with no roadmap trace) |
| Install command | one-liner | `curl -fsSL https://sensei.dev/install.sh \| sh` | run it |
| Pricing tier | Free / Paid / Sponsor | `Paid · $20/active contributor/mo` | choose a plan: free for a solo developer, a personal dōjō and public repositories; an org subscribes for its private repositories (D52) |
| Sponsors link | GitHub Sponsors | `github.com/sponsors/sensei-hq` | sponsor the project |
| Changelog entry | version + date + linked issues | `v0.12.0 · 2026-10-13 · #1247` | read the linked issues, then upgrade |

## Done gate

- A claim audit runs in CI: every feature claim on the site has a roadmap
  trace; any untraced claim fails the build.
- The pricing page matches `features/07-pricing.md` (Free/Paid/Sponsors, no
  Enterprise).
- The install command is the unified one-liner (P1).
- The GitHub Sponsors link works.
- The changelog is generated from tags.

```bash
# Not implemented — placeholder (run in the sensei-hq/website checkout)
npm run claim-audit
```

## Wrong gate

- **A feature is marked "Built" on the site but the roadmap shows 🚧** — claim
  audit regressed.
- **The pricing page shows an Enterprise tier** — tier model regressed.
- **The install command is platform-specific** — P1 unified command regressed.
- **The changelog has hand-written entries not from tags** — changelog source
  regressed.
- **A dropped claim (D-11) reappears** — regression.

## Scenarios

- [`scenarios/website.feature`](scenarios/website.feature) — the claim audit,
  pricing page, install command, changelog (pinned for GitHub Sponsors,
  `NO TEST` / `# gap: D-11` for the rest).
- [`scenarios/website-gaps.feature`](scenarios/website-gaps.feature) — the
  unbuilt claim audit CI (`NO TEST` / `# gap: D-11`).

## Open gaps

- [D-11](../../../../roadmap/06-gaps.md) — website presents unbuilt as shipped
  (one claim already dropped) (P2).
- ~~D-12~~ — **closed 2026-11-03** by `eefc6c119`: every `@kavach/*` at 1.3.2, so
  `@rokkit/*` resolves once at 1.14.0 (#191).
- **Claim audit CI** — not built.

## Related

- [sensei-hq/website](https://github.com/sensei-hq/website) — the SvelteKit marketing site
  (sibling repository, #346).
- [features/07-pricing.md](../../../../features/07-pricing.md) — pricing model.
- [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) — the statuses
  the site must trace.
- [prd/01-vision.md §Business model](../../../01-vision.md) · [prd/03-personas.md §1,5,7](../../../03-personas.md).
- Crates: **none** — this is a SvelteKit app in `sensei-hq/website`.