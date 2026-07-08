# 印 · Pipeline · Project icon inference

**Owner file (proposed):** `crates/senseid/src/analysis/project_icon.rs`
**Called by:** scanner (on project detect / re-scan) → writes
`sensei.projects.icon` (jsonb `{kind, value}`).
**Consumed by:** [[screen/observatory-projects]] cards, project
window header, [[screen/observatory-today]] chip.

## Purpose

Every project card, project window and project chip needs an
icon. Falling back to a generic 場 kanji everywhere makes the app
feel unopinionated — you don't recognise your own projects at a
glance. This pipeline **infers a project icon from the repo** —
README image, `favicon.ico`, `logo.*` file, package.json branding —
and falls back to a domain kanji only when no image can be found.

Kanji is 印 — *sign / mark*.

## Data invariants

- `sensei.projects.icon` is jsonb with shape:
  ```json
  { "kind": "kanji" | "image" | "letter",
    "value": "…",
    "source": "readme" | "favicon" | "logo_file" | "package_branding" | "kanji_map" | "letter_fallback",
    "resolved_at": iso }
  ```
- Icon inference runs at:
  - **Project detect** — first scan, blocks the initial card render
    until best-effort resolution completes (bounded to 500ms —
    misses fall back to kanji, and a background retry queues).
  - **Re-scan** — when the repo tree changes materially (a new
    logo file appears), the inferer re-runs.
- Cached in the same row; not on a separate table. TTL is soft
  (re-run on scan), not time-based.
- **Images are copied into the sensei asset cache**, not
  referenced by remote URL. Retention 30d after last read; URL
  points at the local cached path served by the daemon.

## Inference chain

Ordered — first hit wins. Each step is a small, testable check.

| # | Source | Detection | Icon shape |
|---|---|---|---|
| 1 | README image | Parse README (root, then `docs/README.md`). First `![…](URL)` or `<img src=…>` whose alt/title contains `logo` OR whose src filename matches `logo|brand|banner|mark`. | image, source: `readme` |
| 2 | Repo logo files | Glob for `logo.{png,svg,jpg,webp}`, `icon.{…}`, `assets/logo.{…}`, `.github/logo.{…}`. Prefer svg. | image, source: `logo_file` |
| 3 | favicon | `favicon.ico` in root or `static/`, `public/`, `web/`. If PNG variant exists (`favicon.png`), prefer PNG. | image, source: `favicon` |
| 4 | Package branding | `package.json` `sensei.icon` or `brand.icon` (custom extension) OR `homepage` favicon if fetched offline is not desirable — skip network. | image, source: `package_branding` |
| 5 | Kanji from stack | Map dominant `stack.languages[0]` to a domain kanji (rust→鉄, ts→型, python→蛇, sql→庫, docs→書, tauri→匠, svelte→雪, etc.). See `crates/senseid/src/analysis/kanji_map.rs`. | kanji, source: `kanji_map` |
| 6 | Letter fallback | First letter of `project.name`, uppercase, rendered on a solid tinted background. | letter, source: `letter_fallback` |

## Signals produced

`sensei.projects.icon` — a stable jsonb the UI reads unchanged.

## Done gate

- Every project in `sensei.projects` has a non-null `icon` after
  scanner runs.
- For a project with a README containing a logo, the inferred
  icon is `kind: image, source: readme` and points at a cached
  asset the daemon serves.
- For a rust-heavy project with no repo images, the icon is
  `kind: kanji, value: 鉄, source: kanji_map`.
- For a nameless project with no stack, the icon is
  `kind: letter, value: <first-letter-uppercased>, source: letter_fallback`.
- Rebuilding a project (`rescan`) preserves the icon unless a
  higher-priority source appeared.
- The 500ms bound is honored during first detect — a slow
  README parse doesn't block the projects list.

Optional check:
```
psql -A -t -c "select name, icon->>'kind' as kind,
                          icon->>'source' as source
                 from sensei.projects order by name limit 20" -d sensei

# Any projects still on the kanji_map fallback that HAVE a logo file?
find ~/Developer/some-project -maxdepth 2 -iname 'logo.*' -o -iname 'favicon.*'
```

## Wrong gate

- **Every project resolves to the same letter fallback.** Chain
  short-circuited at step 6; earlier steps aren't firing.
- **Inference network-calls a remote host.** Should never touch
  the network — README image URLs that are external are
  DOWNLOADED once at detect, then cached locally.
- **Card renders a kanji glyph AND an image.** Fallback OR-
  collapsed rather than either-or.
- **Icon changes on every scan when nothing changed.** The
  inferer is non-deterministic OR the cache isn't consulted.
- **Icon `value` is a remote URL** in the DB. Should be a local
  cached path relative to the sensei asset dir.
- **A project with a `logo.svg` in the repo root ended up on
  step 3 (favicon) or later.** Priority order violated.
- **Sensei ships letter-icons for projects that HAVE a logo but
  the logo is inside a nested folder** the glob missed. Extend
  the glob rather than accept the fallback.

## Related

- [[screen/observatory-projects]] — primary consumer of `icon`
- [[screen/project-overview]] — large icon in the project header
- [[screen/observatory-today]] — small icon in the "recent
  sessions" strip
- (Deferred) `pipeline/repo-branding` — a broader "what's the
  brand of this repo" pipeline that could subsume icon + colour +
  vision — not planned yet.
