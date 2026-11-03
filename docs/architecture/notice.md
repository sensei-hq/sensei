---
name: Notice
description: The one component every error, refusal and wait renders through, in sensei and the dōjō — tones, sizes, props and where the copy comes from
date: 2026-11-03
---

# Notice — one component for every error

`packages/ui/Notice.svelte`, exported from `@sensei/ui` and shared by the desktop
app and the dōjō. It is the bootstrap's "needs your hand" screen
(`app/src/routes/(health)/health`) made reusable: an eyebrow, a display headline
with one toned word, a plain sentence, then the Remedy card — a mark and a title,
what to do, an optional script, the actions.

Design board: the "Notice" artboard on the Settings · Dōjō canvas.
Adoption plan: [docs/plan/notice-adoption.md](../plan/notice-adoption.md).

## Tones

Three are `sensei.reason_kind` ([reason codes](reason-codes.md)); the fourth is the
bootstrap's own.

| tone | mark | colour | role | when |
|---|---|---|---|---|
| `fault` | 難 | `text-danger` | `alert` | something broke — always offer a retry or a remedy |
| `refusal` | 止 | `text-warning` | `alert` | someone decided this — say who can change it |
| `action` | 手 | `text-accent` | `status` | fine once the reader acts (run a script, renew, sign in) |
| `normal` | 待 | `text-ink-soft` | `status` | nothing is wrong; it resolves itself |

An unknown tone or reason kind reads as `fault`: a Notice exists to say something
went wrong, and muting an unexpected value is how a failure goes unseen.

## Kanji or icons

The mark follows the glyph mode (#133). Each app's root layout provides it once —
`setGlyphMode(() => mode)` from `@sensei/ui` — and Notice reads it with
`useGlyphMode()`. Kanji mode shows the tone's kanji; icon mode shows that kanji's
vocabulary icon (`@sensei/glyphs`). `mark` / `icon` override both; `mode` overrides
the provided mode. A kanji with no icon stays a kanji — never an empty mark.

## Sizes

| size | use | replaces |
|---|---|---|
| `screen` | a whole screen that could not do its job | ScreenState's error branch |
| `panel` | the card alone, inside a section | Remedy, the dōjō's error boxes |
| `inline` | one line in a row, list or form; "Why?" opens the rest | `<p class="text-danger">…</p>` |

`inline` takes `bare` for a line that already sits in a bordered row.

## Props

`tone`, `size`, `title` (required), `emphasis` + `eyebrow` (screen), `detail`,
`remedy` (+ `remedyTitle`), `actor`, `code`, `script`, `primary` / `secondary`
(`{ label, onclick?, href?, busy? }`), `mark`, `icon`, `mode`, `expanded`, `bare`,
`children` (extra body), and any `data-*` hook, which lands on the root.

## Where the words come from

Never typed into a screen (guidelines §4: copy lives in the state module).

- **A failed API read** → `noticeFromApiError(error, { title, service })`. The screen
  names *what* failed ("Couldn't load tickets"); status 0 tells the reader what to
  check, a 5xx says a retry may clear it, a 4xx shows the server's message as is.
- **A reason code** → `noticeFromReason(row)`. The registry's `summary`, `detail`,
  `remedy` and `actor` are the copy, so a code reads the same on every screen.
- **Anything else** → the state module's `display`/`notice` getter. In the dōjō,
  through paraglide messages.

## Rules

- Title: what happened, sentence case, "Couldn't …" not "Could not …". No
  exclamation marks, no "Oops", no raw error as the headline.
- Say whether anything was saved or lost when the reader would wonder.
- One solid primary, at most one outline secondary. Never a bare "OK".
- The code is small and last. It is for support and logs.
