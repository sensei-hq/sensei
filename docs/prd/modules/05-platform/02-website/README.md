# Website · P2

**Plane:** platform · **Status:** [roadmap/05-platform.md](../../../../roadmap/05-platform.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
engagement / client lead ([persona 5](../../../03-personas.md#5--engagement--client-lead--the-confidentiality-guard)),
client / project stakeholder ([persona 7](../../../03-personas.md#7--client--project-stakeholder--the-narrow-lens))

## Purpose

The website is the **marketing site as a P2 module** — it must **only claim what
is built**; reorg §12 tracks the claims that already slipped (D-11, D-12). The
site is a SvelteKit app (`website/`) that sells the built product, not the
roadmap. The job is **every claim traces to a roadmap status**; the site is
written *after* the roadmap so its claims trace to evidence. The user feels
this as *what the site promises is what the product does.*

> **Status honesty.** The SvelteKit marketing site exists (`website/`). The
> GitHub Sponsors link is live. **One claim was already dropped** (D-11). The
> dependency drift on `@rokkit/*`, `@kavach/*` (D-12) affects the site's own
> stack. The site must be audited against the roadmap before any deploy.

## In scope / out of scope

- In: the marketing site pages (landing, features, pricing, docs, blog,
  changelog); the install command (`curl ... | sh`); the GitHub Sponsors link;
  **claim audit** — every feature claim on the site maps to a roadmap status
  (Built / Partial / Mock-only / Designed / Dormant / Gap).
- Out: the install implementation (P1), the CI/harness (P3), the Dōjō SaaS
  console (T2), the Dōjō web app (`dojo/`).

## Surfaces

- **App** — `website/src/routes/*` (SvelteKit pages); `website/src/lib/components/`
  (shared components); `website/static/` (assets).
- **Data** — roadmap statuses (sourced from `roadmap/*.md` at build time or via
  a content layer); changelog from git tags.
- **CI** — the claim audit runs in CI (planned) — any claim without a roadmap
  trace fails the build.

## Data invariants

- **Lead.** **Every claim traces to the roadmap.** If a page says "X is
  built", `roadmap/03-delivery.md` (or the relevant plane) must show ✅ for X.
- **Lead.** **No roadmap language in marketing.** The site says "sensei does X"
  — not "sensei will do X" or "sensei aims to do X".
- **Lead.** **Pricing page reflects the final model.** Free = individuals/OSS
  forever; Paid = per active contributor; no Enterprise tier; GitHub Sponsors
  optional.
- **Lead.** **Install command is the unified one-liner** (P1) — not
  platform-specific instructions.
- **Lead.** **Changelog is generated from git tags** — not hand-written.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| Feature badge | Built / Partial / Mock-only / Designed | `Observatory · Built` |
| Install command | one-liner | `curl -fsSL https://sensei.dev/install.sh | sh` |
| Pricing tier | Free / Paid / Sponsor | `Paid · $20/active contributor/mo` |
| Sponsors link | GitHub Sponsors | `github.com/sponsors/sensei-hq` |
| Changelog entry | version + date + linked issues | `v0.12.0 · 2026-10-13 · #1247` |

## Done gate

- A claim audit runs in CI: every feature claim on the site has a roadmap
  trace; any untraced claim fails the build.
- The pricing page matches `features/07-pricing.md` (Free/Paid/Sponsors, no
  Enterprise).
- The install command is the unified one-liner (P1).
- The GitHub Sponsors link works.
- The changelog is generated from tags.

```bash
# Not implemented — placeholder
cd website && npm run claim-audit
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
  `NO TEST` / `# gap: D-11, D-12` for the rest).
- [`scenarios/website-gaps.feature`](scenarios/website-gaps.feature) — the
  unbuilt claim audit CI, dependency drift (`NO TEST` / `# gap: D-11, D-12`).

## Open gaps

- [D-11](../../../../roadmap/06-gaps.md) — website presents unbuilt as shipped
  (one claim already dropped) (P2).
- [D-12](../../../../roadmap/06-gaps.md) — dependency drift: `@rokkit/*`,
  `@kavach/*` held back by lockfiles (P3).
- **Claim audit CI** — not built.

## Related

- [website/](../../../../../website/) — the SvelteKit marketing site.
- [features/07-pricing.md](../../../../features/07-pricing.md) — pricing model.
- [roadmap/03-delivery.md](../../../../roadmap/03-delivery.md) — the statuses
  the site must trace.
- [prd/01-vision.md §Business model](../../../01-vision.md) · [prd/03-personas.md §1,5,7](../../../03-personas.md).
- Crates: **none** — this is a SvelteKit app (`website/`).