---
name: zen-sumi-design
description: Use this skill to generate well-branded interfaces and assets for Sensei (brand name Zen-Sumi), either for production or throwaway prototypes/mocks/etc. Contains essential design guidelines, colors, type, fonts, assets, and UI kit components for prototyping a calm, washi-paper-and-sumi-ink aesthetic with restraint as the design language.
user-invocable: true
---

# Zen-Sumi (Sensei) — design skill

Read **`README.md`** in this skill first — it covers visual foundations, content fundamentals, iconography, and the index of available files. Then explore the other files as needed.

## When designing visual artifacts (slides, mocks, throwaway prototypes)

1. **Always start by linking `colors_and_type.css`** — every semantic token, type class, utility, and component class is defined there. Do not invent new tokens. Do not re-define colors inline.
2. **Wrap the root in `<div class="zs">`** so utilities resolve.
3. **Copy `colors_and_type.css` and the contents of `assets/`** into your output project so the artifact is self-contained.
4. **Use the existing scales.** Type has 8 sizes (`xs / sm / base / lg / xl / 2xl / 3xl / 4xl`). Spacing has 9 stops on a 4-base. Colors have 4 paper steps + 4 ink steps + 1 accent + 2 semantic. **If you need a value that isn't in the scale, the design is wrong, not the system.**
5. **Borrow components from the UI kits** in `ui_kits/observatory/` (the desktop app) and `ui_kits/site/` (the marketing site). The patterns are: kanji + eyebrow + display headline + body paragraph; hairline-divided lists; insight cards with a meaningful badge instead of a number; window chrome with traffic lights.

## When working on production code

Read `README.md` and `colors_and_type.css`. The CSS file is drop-in — it scopes everything under `.zs`, so it won't collide with existing styles. Use the semantic tokens and the utility classes; treat the component classes (`zs-btn`, `zs-card`, `zs-input`) as references, not as a hard component library — if you have an existing React framework, recreate them inside your framework using the same tokens.

## Voice & content rules (the non-negotiables)

- **No exclamation marks.** Ever.
- **No emoji.** Use a kanji or a sentence instead.
- **Sentence case.** Never title case.
- **Lowercase "sensei"** when sensei talks about itself in third person.
- **Periods.** Not exclamation marks. (Worth repeating.)
- **No "AI-powered", "supercharge", "unlock", "let's get started!"** — the system actively avoids marketing-speak.
- **Numbers are meaningful** — `3rd time` is better than `1,247 patterns recognized`.

## If the user invokes this skill without context

Ask what they want to build. Likely options for this brand:
- A new in-app screen or component for the desktop Observatory
- A marketing-site section or page
- A presentation deck (use 1920×1080, washi paper background, kanji accents)
- A README, doc, or onboarding flow

Then ask the design questions (audience, fidelity, options) and act as an expert designer who outputs HTML artifacts or production code, depending on the need.

## Quick reference

### Required token usage
```css
/* Use these. Never raw hex. */
background: var(--paper);       /* page */
background: var(--paper-2);     /* cards */
color: var(--ink);              /* primary text */
color: var(--ink-2);            /* body */
color: var(--ink-3);            /* meta */
color: var(--accent);           /* vermillion — rationed */
border: var(--hairline);        /* dividers */
```

### Required type classes
```html
<h1 class="zs-h1">...</h1>      <!-- 28px Fraunces, tight -->
<h2 class="zs-h2">...</h2>      <!-- 22px Fraunces -->
<div class="zs-display">...</div>   <!-- 40px Fraunces -->
<div class="zs-hero">...</div>      <!-- 56px Fraunces hero -->
<div class="zs-eyebrow">...</div>   <!-- 11px uppercase letterspaced -->
<span class="zs-kanji">観</span>    <!-- Mincho serif for kanji -->
<span class="zs-mono">s-2891</span> <!-- JetBrains Mono -->
```

### Anti-patterns
- Lucide / Heroicons / Material / Feather / Font Awesome → use kanji or the SVG glyph set
- Generic stock photos → use product mockups
- Colored rounded-corner squares behind icons → never
- Gradient backgrounds → never (one rare exception: large kanji watermark)
- Card with colored left-border accent → never
- "Loading…" spinners → use copy ("Still listening.")
