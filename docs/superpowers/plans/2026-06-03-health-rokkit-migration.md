# `/health` Rokkit Migration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate `sensei/app/src/routes/(health)/health/**` off the deprecated Rokkit z-scale onto the canonical 24 named tokens, extract 6 reusable primitives, and rebuild the screen against `docs/mockups/Sensei/lib/bootstrap-splash.jsx` while preserving all `boot-flow.spec.ts` behaviour.

**Architecture:** Token bridge is `rokkit.config.js` `overrides:` only — no `shortcuts:`, no `rules:`. State (`health-state.svelte.ts`) owns every collection derivation, status-to-copy mapping, and action. Components (`Wordmark`, `KanjiHeader`, `StatusDisc`, `Spinner` in `$lib/components/`; `StatusIndicator`, `GateRow` co-located in the route) are pure templates that take props, render markup, apply utility classes. The skin re-alignment (`primary → kami/sumi`, `accent → shu`) happens **last** so existing `text-primary-z*` references render correct vermillion throughout the migration.

**Tech Stack:** SvelteKit (Svelte 5 runes), Tauri, UnoCSS + `@rokkit/unocss` preset, Vitest + jsdom for component specs, `mountComponent` helper from `$lib/test-mount.ts`, Playwright e2e behaviour tests.

**Companion docs:**
- Spec: `sensei/docs/superpowers/specs/2026-06-02-health-rokkit-migration-design.md`
- Guidelines: `sensei/docs/design/frontend-svelte-guidelines.md`
- Mockup: `sensei/docs/mockups/Sensei/lib/bootstrap-splash.jsx`

---

## Task 1: Add `overrides:` block — paper, ink, accent, primary, status dark shifts

Lock the canonical 24 named tokens to the design system's intended shades. Skin
mapping stays unchanged so existing `text-primary-z*` references in unmigrated
code render vermillion correctly until Task 12.

**Files:**
- Modify: `sensei/app/rokkit.config.js`

- [ ] **Step 1: Replace the `overrides:` block**

Current overrides (one entry):
```js
overrides: {
  "paper-edge": { light: "kami.400", dark: "sumi.100" },
},
```

Replace with the full block:
```js
overrides: {
  // ── Surface (paper) ──────────────────────────────────────────
  paper:        { light: "kami.100", dark: "sumi.50"  },
  "paper-soft": { light: "kami.200", dark: "sumi.100" },
  "paper-mute": { light: "kami.300", dark: "sumi.200" },
  "paper-edge": { light: "kami.400", dark: "sumi.100" },  // etched hairline in dark

  // ── Ink (text-zone shades; sumi.600-900 is the two-pole text half) ─
  ink:          { light: "kami.900", dark: "sumi.900" },
  "ink-soft":   { light: "kami.700", dark: "sumi.800" },
  "ink-mute":   { light: "kami.500", dark: "sumi.700" },
  "ink-faint":  { light: "kami.300", dark: "sumi.600" },

  // ── Accent — vermillion (design system: --accent: var(--shu-500)) ─
  // Skin role for `accent` is still `fuji`; override forces shu values
  // so `bg-accent`, `text-accent`, `border-accent` resolve to vermillion.
  // Skin re-alignment happens in Task 12.
  accent:        { light: "shu.500", dark: "shu.400" },
  "accent-soft": { light: "shu.100", dark: "shu.200" },

  // ── Primary — ink-colored (design system: --primary: var(--ink)) ─
  // For the future ink-on-paper CTA button. Existing text-primary-z*
  // (z-scale) still resolves via the skin (primary: shu) = vermillion.
  primary:      { light: "kami.900", dark: "sumi.900" },
  "on-primary": { light: "kami.100", dark: "sumi.50"  },

  // ── Status — lighten for legibility in dark mode (shade 400 vs 500) ─
  success:      { light: "hisui.500",  dark: "hisui.400"  },
  warning:      { light: "kohaku.500", dark: "kohaku.400" },
  danger:       { light: "beni.500",   dark: "beni.400"   },
  info:         { light: "ai.500",     dark: "ai.400"     },
},
```

- [ ] **Step 2: Verify build still works**

Run: `cd sensei/app && bun run check`
Expected: zero errors. `bun run lint` likewise.

- [ ] **Step 3: Smoke-test in dev**

Run: `cd sensei && make app-dev`
Navigate to `/health`. Confirm:
- The screen renders (no broken CSS / missing utility crashes).
- Vermillion still appears where `text-primary-z*` is used in current code (Header kanji, Hero spinner, Ledger badges) — because the skin's `primary: 'shu'` is unchanged.
- Light + dark mode both render. Toggle via the existing mode mechanism in dev (or inject `data-mode="dark"` via DevTools).

Note: the look may shift subtly because `paper`/`ink` named tokens now have explicit values. This is expected. Verify nothing is *broken* (white-on-white, illegible).

- [ ] **Step 4: Commit**

```bash
cd sensei
git add app/rokkit.config.js
git commit -m "feat(app): add named-token overrides for paper/ink/accent/primary/status"
```

---

## Task 2: `Spinner` primitive (lib/components)

Small standalone spinner used by `StatusDisc`. Replaces two inline `<style>` blocks.

**Files:**
- Create: `sensei/app/src/lib/components/Spinner.svelte`
- Create: `sensei/app/src/lib/components/Spinner.harness.svelte`
- Create: `sensei/app/src/lib/components/Spinner.spec.svelte.ts`
- Modify: `sensei/app/src/lib/components/index.ts`

- [ ] **Step 1: Write the failing test**

Create `sensei/app/src/lib/components/Spinner.spec.svelte.ts`:

```ts
// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import SpinnerHarness from './Spinner.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Spinner', () => {
  it('renders the spinner element', () => {
    const m = mountComponent(SpinnerHarness, { size: 10, tone: 'accent' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="spinner"]') as HTMLElement;
    expect(el).toBeTruthy();
  });

  it('applies tone="accent" class', () => {
    const m = mountComponent(SpinnerHarness, { size: 10, tone: 'accent' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="spinner"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-accent\b/);
  });

  it('applies tone="success" class', () => {
    const m = mountComponent(SpinnerHarness, { size: 14, tone: 'success' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="spinner"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-success\b/);
  });

  it('uses provided size', () => {
    const m = mountComponent(SpinnerHarness, { size: 12, tone: 'accent' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="spinner"]') as HTMLElement;
    expect(el.style.width).toBe('12px');
    expect(el.style.height).toBe('12px');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd sensei/app && bun run test -- Spinner`
Expected: FAIL (Spinner.svelte and Spinner.harness.svelte do not exist).

- [ ] **Step 3: Create the harness**

Create `sensei/app/src/lib/components/Spinner.harness.svelte`:

```svelte
<script lang="ts">
  import Spinner from './Spinner.svelte';

  let { size = 10, tone = 'accent' }: {
    size?: number;
    tone?: 'accent' | 'success' | 'ink';
  } = $props();
</script>

<Spinner {size} {tone} />
```

- [ ] **Step 4: Create the component**

Create `sensei/app/src/lib/components/Spinner.svelte`:

```svelte
<script lang="ts">
  interface Props {
    size?: number;
    tone?: 'accent' | 'success' | 'ink';
  }
  let { size = 10, tone = 'accent' }: Props = $props();

  const toneClass = $derived(
    tone === 'success' ? 'border-success'
    : tone === 'ink'   ? 'border-ink'
    :                    'border-accent',
  );
</script>

<span
  data-component="spinner"
  class="block rounded-full border-[1.5px] border-t-transparent {toneClass}"
  style="width: {size}px; height: {size}px; animation: spinner-spin 0.9s linear infinite;"
  aria-hidden="true"
></span>

<style>
  @keyframes spinner-spin {
    to { transform: rotate(360deg); }
  }
</style>
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd sensei/app && bun run test -- Spinner`
Expected: PASS (all 4 tests green).

- [ ] **Step 6: Add to the barrel export**

Modify `sensei/app/src/lib/components/index.ts` — add at the end:

```ts
export { default as Spinner } from './Spinner.svelte';
```

- [ ] **Step 7: Verify build**

Run: `cd sensei/app && bun run check && bun run lint`
Expected: zero errors.

- [ ] **Step 8: Commit**

```bash
cd sensei
git add app/src/lib/components/Spinner.svelte \
        app/src/lib/components/Spinner.harness.svelte \
        app/src/lib/components/Spinner.spec.svelte.ts \
        app/src/lib/components/index.ts
git commit -m "feat(app): add Spinner primitive"
```

---

## Task 3: `StatusDisc` primitive (lib/components)

20px / 32px circular disc that shows check (ready), spinner (busy), `?` glyph (failed/blocked), or empty (pending). Replaces inline `.hero-disc` (Hero) and per-row inline disc (Ledger).

**Files:**
- Create: `sensei/app/src/lib/components/StatusDisc.svelte`
- Create: `sensei/app/src/lib/components/StatusDisc.harness.svelte`
- Create: `sensei/app/src/lib/components/StatusDisc.spec.svelte.ts`
- Modify: `sensei/app/src/lib/components/index.ts`

- [ ] **Step 1: Write the failing test**

Create `sensei/app/src/lib/components/StatusDisc.spec.svelte.ts`:

```ts
// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import StatusDiscHarness from './StatusDisc.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('StatusDisc', () => {
  it('renders the disc element', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'pending' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="status-disc"]')).toBeTruthy();
  });

  it('renders check glyph when status="ready"', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'ready' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-success\b/);
    expect(el.querySelector('svg')).toBeTruthy();
  });

  it('renders spinner when status="checking"', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'checking' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-accent\b/);
    expect(el.querySelector('[data-component="spinner"]')).toBeTruthy();
  });

  it('renders spinner when status="installing"', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'installing' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.querySelector('[data-component="spinner"]')).toBeTruthy();
  });

  it('renders question kanji when status="failed"', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'failed' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-accent\b/);
    expect(el.textContent).toContain('?');
  });

  it('uses muted border when status="pending"', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'pending' });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.className).toMatch(/\bborder-ink-faint\b/);
  });

  it('applies provided size (default 20)', () => {
    const m = mountComponent(StatusDiscHarness, { status: 'pending', size: 32 });
    cleanup.push(m.destroy);
    const el = m.container.querySelector('[data-component="status-disc"]') as HTMLElement;
    expect(el.style.width).toBe('32px');
    expect(el.style.height).toBe('32px');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd sensei/app && bun run test -- StatusDisc`
Expected: FAIL (component does not exist).

- [ ] **Step 3: Create the harness**

Create `sensei/app/src/lib/components/StatusDisc.harness.svelte`:

```svelte
<script lang="ts">
  import StatusDisc from './StatusDisc.svelte';
  import type { ComponentStatus } from '$lib/health-types.js';

  let { status, size = 20 }: {
    status: ComponentStatus;
    size?: number;
  } = $props();
</script>

<StatusDisc {status} {size} />
```

- [ ] **Step 4: Create the component**

Create `sensei/app/src/lib/components/StatusDisc.svelte`:

```svelte
<script lang="ts">
  import type { ComponentStatus } from '$lib/health-types.js';
  import Spinner from './Spinner.svelte';

  interface Props {
    status: ComponentStatus;
    size?: number;
  }
  let { status, size = 20 }: Props = $props();

  const borderClass = $derived(
    status === 'ready'                                ? 'border-success'
    : status === 'checking' || status === 'installing'? 'border-accent'
    : status === 'failed'                              ? 'border-accent'
    :                                                    'border-ink-faint',
  );

  const innerSize = $derived(size >= 32 ? 14 : size >= 24 ? 11 : 10);
  const strokeWidth = $derived(size >= 32 ? 2 : 1.5);
</script>

<span
  data-component="status-disc"
  data-status={status}
  class="inline-flex items-center justify-center rounded-full bg-paper border-[1.5px] shrink-0 {borderClass}"
  style="width: {size}px; height: {size}px;"
>
  {#if status === 'ready'}
    <svg width={innerSize} height={innerSize} viewBox="0 0 10 10" fill="none" aria-hidden="true">
      <path d="M2 5.2 L4.2 7.2 L8 3" stroke="currentColor"
            stroke-width={strokeWidth} stroke-linecap="round" stroke-linejoin="round"
            class="text-success" />
    </svg>
  {:else if status === 'checking' || status === 'installing'}
    <Spinner size={innerSize} tone="accent" />
  {:else if status === 'failed'}
    <span class="font-kanji text-accent leading-none" style="font-size: {Math.round(size * 0.6)}px;">?</span>
  {/if}
</span>
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd sensei/app && bun run test -- StatusDisc`
Expected: PASS (all 7 tests green).

- [ ] **Step 6: Add to barrel export**

Modify `sensei/app/src/lib/components/index.ts` — add:

```ts
export { default as StatusDisc } from './StatusDisc.svelte';
```

- [ ] **Step 7: Commit**

```bash
cd sensei
git add app/src/lib/components/StatusDisc.svelte \
        app/src/lib/components/StatusDisc.harness.svelte \
        app/src/lib/components/StatusDisc.spec.svelte.ts \
        app/src/lib/components/index.ts
git commit -m "feat(app): add StatusDisc primitive"
```

---

## Task 4: `Wordmark` primitive (lib/components)

The `先生 Sensei` ranked mark. Used in the Header (size `md`) and the all-green state (size `lg`).

**Files:**
- Create: `sensei/app/src/lib/components/Wordmark.svelte`
- Create: `sensei/app/src/lib/components/Wordmark.harness.svelte`
- Create: `sensei/app/src/lib/components/Wordmark.spec.svelte.ts`
- Modify: `sensei/app/src/lib/components/index.ts`

- [ ] **Step 1: Write the failing test**

Create `sensei/app/src/lib/components/Wordmark.spec.svelte.ts`:

```ts
// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import WordmarkHarness from './Wordmark.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('Wordmark', () => {
  it('renders the kanji 先生 and the word Sensei', () => {
    const m = mountComponent(WordmarkHarness, { size: 'md' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('先生');
    expect(m.container.textContent).toContain('Sensei');
  });

  it('uses accent color for the kanji', () => {
    const m = mountComponent(WordmarkHarness, { size: 'md' });
    cleanup.push(m.destroy);
    const kanji = m.container.querySelector('[data-component="wordmark-kanji"]') as HTMLElement;
    expect(kanji.className).toMatch(/\btext-accent\b/);
    expect(kanji.className).toMatch(/\bfont-kanji\b/);
  });

  it('uses the display font for the word', () => {
    const m = mountComponent(WordmarkHarness, { size: 'md' });
    cleanup.push(m.destroy);
    const word = m.container.querySelector('[data-component="wordmark-word"]') as HTMLElement;
    expect(word.className).toMatch(/\bfont-display\b/);
  });

  it('applies sm size classes', () => {
    const m = mountComponent(WordmarkHarness, { size: 'sm' });
    cleanup.push(m.destroy);
    const kanji = m.container.querySelector('[data-component="wordmark-kanji"]') as HTMLElement;
    expect(kanji.className).toMatch(/\btext-lg\b/);
  });

  it('applies lg size classes', () => {
    const m = mountComponent(WordmarkHarness, { size: 'lg' });
    cleanup.push(m.destroy);
    const kanji = m.container.querySelector('[data-component="wordmark-kanji"]') as HTMLElement;
    expect(kanji.className).toMatch(/\btext-3xl\b/);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd sensei/app && bun run test -- Wordmark`
Expected: FAIL.

- [ ] **Step 3: Create harness + component**

Create `sensei/app/src/lib/components/Wordmark.harness.svelte`:

```svelte
<script lang="ts">
  import Wordmark from './Wordmark.svelte';
  let { size = 'md' }: { size?: 'sm' | 'md' | 'lg' } = $props();
</script>

<Wordmark {size} />
```

Create `sensei/app/src/lib/components/Wordmark.svelte`:

```svelte
<script lang="ts">
  interface Props { size?: 'sm' | 'md' | 'lg'; }
  let { size = 'md' }: Props = $props();

  const kanjiSizeClass = $derived(
    size === 'sm' ? 'text-lg'
    : size === 'lg' ? 'text-3xl'
    :                 'text-2xl',
  );
  const wordSizeClass = $derived(
    size === 'sm' ? 'text-sm'
    : size === 'lg' ? 'text-xl'
    :                 'text-base',
  );
</script>

<div class="flex items-baseline gap-2">
  <span
    data-component="wordmark-kanji"
    class="font-kanji text-accent leading-none {kanjiSizeClass}"
  >先生</span>
  <span
    data-component="wordmark-word"
    class="font-display font-normal tracking-tight text-ink {wordSizeClass}"
  >Sensei</span>
</div>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd sensei/app && bun run test -- Wordmark`
Expected: PASS.

- [ ] **Step 5: Add to barrel + commit**

Modify `sensei/app/src/lib/components/index.ts`:

```ts
export { default as Wordmark } from './Wordmark.svelte';
```

```bash
cd sensei
git add app/src/lib/components/Wordmark.svelte \
        app/src/lib/components/Wordmark.harness.svelte \
        app/src/lib/components/Wordmark.spec.svelte.ts \
        app/src/lib/components/index.ts
git commit -m "feat(app): add Wordmark primitive"
```

---

## Task 5: `KanjiHeader` primitive (lib/components)

`kanji + eyebrow + title + optional right slot` — the canonical mockup pattern. Used in the Hero (kanji `支` + eyebrow `foundation` + dynamic title + right `<StatusDisc size=32>`).

**Files:**
- Create: `sensei/app/src/lib/components/KanjiHeader.svelte`
- Create: `sensei/app/src/lib/components/KanjiHeader.harness.svelte`
- Create: `sensei/app/src/lib/components/KanjiHeader.spec.svelte.ts`
- Modify: `sensei/app/src/lib/components/index.ts`

- [ ] **Step 1: Write the failing test**

Create `sensei/app/src/lib/components/KanjiHeader.spec.svelte.ts`:

```ts
// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import KanjiHeaderHarness from './KanjiHeader.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('KanjiHeader', () => {
  it('renders kanji, eyebrow, and title', () => {
    const m = mountComponent(KanjiHeaderHarness, {
      kanji: '支',
      eyebrow: 'foundation',
      title: 'Checking components',
      withRight: false,
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('支');
    expect(m.container.textContent).toContain('foundation');
    expect(m.container.textContent).toContain('Checking components');
  });

  it('eyebrow uses uppercase + ink-mute', () => {
    const m = mountComponent(KanjiHeaderHarness, {
      kanji: '支', eyebrow: 'foundation', title: 'X', withRight: false,
    });
    cleanup.push(m.destroy);
    const eyebrow = m.container.querySelector('[data-component="kanji-header-eyebrow"]') as HTMLElement;
    expect(eyebrow.className).toMatch(/\buppercase\b/);
    expect(eyebrow.className).toMatch(/\btext-ink-mute\b/);
  });

  it('kanji uses font-kanji + text-accent', () => {
    const m = mountComponent(KanjiHeaderHarness, {
      kanji: '支', eyebrow: 'x', title: 'X', withRight: false,
    });
    cleanup.push(m.destroy);
    const kanji = m.container.querySelector('[data-component="kanji-header-kanji"]') as HTMLElement;
    expect(kanji.className).toMatch(/\bfont-kanji\b/);
    expect(kanji.className).toMatch(/\btext-accent\b/);
  });

  it('renders the right slot when provided', () => {
    const m = mountComponent(KanjiHeaderHarness, {
      kanji: '支', eyebrow: 'x', title: 'X', withRight: true,
    });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-test="harness-right"]')).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd sensei/app && bun run test -- KanjiHeader`
Expected: FAIL.

- [ ] **Step 3: Create harness + component**

Create `sensei/app/src/lib/components/KanjiHeader.harness.svelte`:

```svelte
<script lang="ts">
  import KanjiHeader from './KanjiHeader.svelte';
  let { kanji, eyebrow, title, withRight = false }: {
    kanji: string; eyebrow: string; title: string; withRight?: boolean;
  } = $props();
</script>

<KanjiHeader {kanji} {eyebrow}>
  {#snippet title()}{title}{/snippet}
  {#if withRight}
    {#snippet right()}<span data-test="harness-right">R</span>{/snippet}
  {/if}
</KanjiHeader>
```

Create `sensei/app/src/lib/components/KanjiHeader.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    kanji: string;
    eyebrow: string;
    title: Snippet;
    right?: Snippet;
  }
  let { kanji, eyebrow, title, right }: Props = $props();
</script>

<section class="flex items-start gap-3">
  <span
    data-component="kanji-header-kanji"
    class="font-kanji text-accent text-2xl leading-none shrink-0"
  >{kanji}</span>

  <div class="flex-1 min-w-0">
    <div
      data-component="kanji-header-eyebrow"
      class="text-xs tracking-wide uppercase text-ink-mute font-medium leading-none"
    >{eyebrow}</div>
    <div
      data-component="kanji-header-title"
      class="font-display text-lg font-normal text-ink leading-snug mt-1"
    >{@render title()}</div>
  </div>

  {#if right}
    <div class="shrink-0">{@render right()}</div>
  {/if}
</section>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd sensei/app && bun run test -- KanjiHeader`
Expected: PASS.

- [ ] **Step 5: Add to barrel + commit**

Modify `sensei/app/src/lib/components/index.ts`:

```ts
export { default as KanjiHeader } from './KanjiHeader.svelte';
```

```bash
cd sensei
git add app/src/lib/components/KanjiHeader.svelte \
        app/src/lib/components/KanjiHeader.harness.svelte \
        app/src/lib/components/KanjiHeader.spec.svelte.ts \
        app/src/lib/components/index.ts
git commit -m "feat(app): add KanjiHeader primitive"
```

---

## Task 6: `StatusIndicator` (screen-local)

Mono-uppercase label + 20px `StatusDisc` inline. Replaces Hero `.hero-disc` row treatment and Ledger badge+disc combo. Lives in `(health)/health/` because no second consumer yet.

**Files:**
- Create: `sensei/app/src/routes/(health)/health/StatusIndicator.svelte`
- Create: `sensei/app/src/routes/(health)/health/StatusIndicator.harness.svelte`
- Create: `sensei/app/src/routes/(health)/health/StatusIndicator.spec.svelte.ts`

- [ ] **Step 1: Write the failing test**

Create `sensei/app/src/routes/(health)/health/StatusIndicator.spec.svelte.ts`:

```ts
// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import StatusIndicatorHarness from './StatusIndicator.harness.svelte';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

describe('StatusIndicator', () => {
  it('renders no label when status="pending" and no label provided', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'pending' });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]');
    expect(label).toBeNull();
  });

  it('renders the disc for every status', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'pending' });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-component="status-disc"]')).toBeTruthy();
  });

  it('renders "checking" label for status="checking"', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'checking' });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]') as HTMLElement;
    expect(label.textContent?.trim()).toBe('checking');
    expect(label.className).toMatch(/\btext-accent\b/);
    expect(label.className).toMatch(/\bfont-mono\b/);
    expect(label.className).toMatch(/\buppercase\b/);
  });

  it('renders "ready" label for status="ready"', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'ready' });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]') as HTMLElement;
    expect(label.textContent?.trim()).toBe('ready');
    expect(label.className).toMatch(/\btext-success\b/);
  });

  it('renders "blocked" label for status="failed"', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'failed' });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]') as HTMLElement;
    expect(label.textContent?.trim()).toBe('blocked');
    expect(label.className).toMatch(/\btext-accent\b/);
  });

  it('uses provided label for installing (installingVerb override)', () => {
    const m = mountComponent(StatusIndicatorHarness, { status: 'installing', label: 'starting' });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]') as HTMLElement;
    expect(label.textContent?.trim()).toBe('starting');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd sensei/app && bun run test -- StatusIndicator`
Expected: FAIL.

- [ ] **Step 3: Create harness + component**

Create `sensei/app/src/routes/(health)/health/StatusIndicator.harness.svelte`:

```svelte
<script lang="ts">
  import StatusIndicator from './StatusIndicator.svelte';
  import type { ComponentStatus } from '$lib/health-types.js';

  let { status, label }: {
    status: ComponentStatus;
    label?: string;
  } = $props();
</script>

<StatusIndicator {status} {label} />
```

Create `sensei/app/src/routes/(health)/health/StatusIndicator.svelte`:

```svelte
<script lang="ts">
  import type { ComponentStatus } from '$lib/health-types.js';
  import { StatusDisc } from '$lib/components';

  interface Props {
    status: ComponentStatus;
    label?: string;
  }
  let { status, label }: Props = $props();

  const defaultLabel = $derived(
    status === 'ready'    ? 'ready'
    : status === 'failed' ? 'blocked'
    : status === 'checking'   ? 'checking'
    : status === 'installing' ? 'installing'
    :                            null,  // pending — no label
  );

  const displayLabel = $derived(label ?? defaultLabel);

  const labelTone = $derived(
    status === 'ready'                              ? 'text-success'
    : status === 'checking' || status === 'installing' ? 'text-accent'
    : status === 'failed'                            ? 'text-accent'
    :                                                  'text-ink-faint',
  );
</script>

<div class="inline-flex items-center gap-2 shrink-0">
  {#if displayLabel}
    <span
      data-component="status-indicator-label"
      class="font-mono text-xs uppercase tracking-wide leading-none {labelTone}"
    >{displayLabel}</span>
  {/if}
  <StatusDisc {status} size={20} />
</div>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd sensei/app && bun run test -- StatusIndicator`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd sensei
git add app/src/routes/\(health\)/health/StatusIndicator.svelte \
        app/src/routes/\(health\)/health/StatusIndicator.harness.svelte \
        app/src/routes/\(health\)/health/StatusIndicator.spec.svelte.ts
git commit -m "feat(app): add StatusIndicator (mono label + StatusDisc)"
```

---

## Task 7: `GateRow` (screen-local)

One ledger row: kanji numeral + name+detail + zen italic line + `StatusIndicator`. Matches `SplashGateRow` in `bootstrap-splash.jsx`.

**Files:**
- Create: `sensei/app/src/routes/(health)/health/GateRow.svelte`
- Create: `sensei/app/src/routes/(health)/health/GateRow.harness.svelte`
- Create: `sensei/app/src/routes/(health)/health/GateRow.spec.svelte.ts`

- [ ] **Step 1: Write the failing test**

Create `sensei/app/src/routes/(health)/health/GateRow.spec.svelte.ts`:

```ts
// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import GateRowHarness from './GateRow.harness.svelte';
import type { Component } from '$lib/health-types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

function makeGate(overrides: Partial<Component> = {}): Component {
  return {
    id: 'postgres',
    label: 'PostgreSQL',
    detail: 'storage · @16',
    note: null,
    status: 'pending',
    version: null,
    installingVerb: 'installing',
    description: 'A still pond where memories settle.',
    ...overrides,
  };
}

describe('GateRow', () => {
  it('renders kanji numeral, name, detail, description', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate(), numeral: '二' });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('二');
    expect(m.container.textContent).toContain('PostgreSQL');
    expect(m.container.textContent).toContain('storage · @16');
    expect(m.container.textContent).toContain('A still pond where memories settle.');
  });

  it('shows description in italic with ink-soft', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate(), numeral: '二' });
    cleanup.push(m.destroy);
    const desc = m.container.querySelector('[data-component="gate-row-description"]') as HTMLElement;
    expect(desc.className).toMatch(/\bitalic\b/);
    expect(desc.className).toMatch(/\btext-ink-soft\b/);
  });

  it('numeral is success-colored when status="ready"', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate({ status: 'ready' }), numeral: '一' });
    cleanup.push(m.destroy);
    const numeral = m.container.querySelector('[data-component="gate-row-numeral"]') as HTMLElement;
    expect(numeral.className).toMatch(/\btext-success\b/);
  });

  it('numeral is accent-colored when status="failed"', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate({ status: 'failed' }), numeral: '一' });
    cleanup.push(m.destroy);
    const numeral = m.container.querySelector('[data-component="gate-row-numeral"]') as HTMLElement;
    expect(numeral.className).toMatch(/\btext-accent\b/);
  });

  it('numeral is muted when status="pending"', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate({ status: 'pending' }), numeral: '一' });
    cleanup.push(m.destroy);
    const numeral = m.container.querySelector('[data-component="gate-row-numeral"]') as HTMLElement;
    expect(numeral.className).toMatch(/\btext-ink-faint\b/);
  });

  it('row dims when pending', () => {
    const m = mountComponent(GateRowHarness, { gate: makeGate({ status: 'pending' }), numeral: '一' });
    cleanup.push(m.destroy);
    const row = m.container.querySelector('[data-component="gate-row"]') as HTMLElement;
    expect(row.style.opacity).toBe('0.5');
  });

  it('passes installingVerb as StatusIndicator label when status="installing"', () => {
    const m = mountComponent(GateRowHarness, {
      gate: makeGate({ status: 'installing', installingVerb: 'configuring' }),
      numeral: '五',
    });
    cleanup.push(m.destroy);
    const label = m.container.querySelector('[data-component="status-indicator-label"]') as HTMLElement;
    expect(label.textContent?.trim()).toBe('configuring');
  });

  it('shows version when present', () => {
    const m = mountComponent(GateRowHarness, {
      gate: makeGate({ version: '16.4', status: 'ready' }),
      numeral: '二',
    });
    cleanup.push(m.destroy);
    expect(m.container.textContent).toContain('16.4');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd sensei/app && bun run test -- GateRow`
Expected: FAIL.

- [ ] **Step 3: Create harness + component**

Create `sensei/app/src/routes/(health)/health/GateRow.harness.svelte`:

```svelte
<script lang="ts">
  import GateRow from './GateRow.svelte';
  import type { Component } from '$lib/health-types.js';

  let { gate, numeral }: { gate: Component; numeral: string } = $props();
</script>

<GateRow {gate} {numeral} />
```

Create `sensei/app/src/routes/(health)/health/GateRow.svelte`:

```svelte
<script lang="ts">
  import type { Component } from '$lib/health-types.js';
  import StatusIndicator from './StatusIndicator.svelte';

  interface Props {
    gate: Component;
    numeral: string;
  }
  let { gate, numeral }: Props = $props();

  const numeralTone = $derived(
    gate.status === 'ready'                                ? 'text-success'
    : gate.status === 'failed'                              ? 'text-accent'
    : gate.status === 'checking' || gate.status === 'installing' ? 'text-ink-mute'
    :                                                          'text-ink-faint',
  );

  const indicatorLabel = $derived(
    gate.status === 'installing' ? gate.installingVerb : undefined,
  );
</script>

<div
  data-component="gate-row"
  data-gate-id={gate.id}
  class="border-b border-paper-edge py-2.5"
  style="opacity: {gate.status === 'pending' ? 0.5 : 1};"
>
  <div class="grid grid-cols-[22px_1fr_auto] items-center gap-3">
    <span
      data-component="gate-row-numeral"
      class="font-kanji text-lg text-center leading-none {numeralTone}"
    >{numeral}</span>

    <div class="min-w-0 flex flex-col gap-1">
      <div class="flex items-baseline gap-2 flex-wrap leading-tight">
        <span class="font-display text-sm font-medium text-ink">{gate.label}</span>
        {#if gate.detail}
          <span class="text-xs text-ink-faint">· {gate.detail}</span>
        {/if}
        {#if gate.version}
          <span class="font-mono text-xs text-ink-mute">{gate.version}</span>
        {/if}
      </div>
      <div
        data-component="gate-row-description"
        class="italic text-xs text-ink-soft leading-snug"
      >{gate.description}</div>
    </div>

    <StatusIndicator status={gate.status} label={indicatorLabel} />
  </div>
</div>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd sensei/app && bun run test -- GateRow`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
cd sensei
git add app/src/routes/\(health\)/health/GateRow.svelte \
        app/src/routes/\(health\)/health/GateRow.harness.svelte \
        app/src/routes/\(health\)/health/GateRow.spec.svelte.ts
git commit -m "feat(app): add GateRow with kanji numeral + zen prose"
```

---

## Task 8: `health-state.svelte.ts` extensions

Move every collection derivation and status→copy mapping into state. Add `gates`, `total`, `readyCount`, `activeLabel`, `firstBlockedIdx`, `display`, and `retry(id)`.

**Files:**
- Modify: `sensei/app/src/lib/health-state.svelte.ts`
- Modify: `sensei/app/src/lib/health-state.spec.svelte.ts`

- [ ] **Step 1: Read current state file**

Read `sensei/app/src/lib/health-state.svelte.ts` end-to-end to understand the existing `HealthState` class shape, what `$state` fields exist, and the existing constructor / `init()` / `verify()` / `apply()` methods. Stay backward compatible — no field removals.

- [ ] **Step 2: Write the failing tests**

Modify `sensei/app/src/lib/health-state.spec.svelte.ts` — append these tests inside the existing `describe('HealthState', ...)` (or add as a sibling describe block):

```ts
describe('HealthState — derivations', () => {
  it('gates returns packageManager + components in that order', () => {
    const s = makeState({  /* helper: populates packageManager + components */ });
    expect(s.gates[0].id).toBe(s.packageManager.id);
    expect(s.gates.length).toBe(1 + s.components.length);
  });

  it('total counts all gates', () => {
    const s = makeState({});
    expect(s.total).toBe(6);  // pm + 5 components
  });

  it('readyCount counts gates with status="ready"', () => {
    const s = makeState({ readyIds: ['homebrew', 'postgres', 'ollama'] });
    expect(s.readyCount).toBe(3);
  });

  it('activeLabel returns label of first installing/checking gate', () => {
    const s = makeState({
      readyIds: ['homebrew', 'postgres'],
      installingId: 'ollama',
    });
    expect(s.activeLabel).toBe('Ollama');
  });

  it('activeLabel is empty when no gate is active', () => {
    const s = makeState({ readyIds: ['homebrew', 'postgres', 'ollama', 'sensei', 'database', 'daemon'] });
    expect(s.activeLabel).toBe('');
  });

  it('firstBlockedIdx returns index of first failed gate, -1 when none', () => {
    const s1 = makeState({ failedId: 'ollama' });  // index 2 in gates
    expect(s1.firstBlockedIdx).toBe(2);

    const s2 = makeState({});
    expect(s2.firstBlockedIdx).toBe(-1);
  });
});

describe('HealthState — display', () => {
  it('checking status produces "starting" eyebrow', () => {
    const s = makeState({ status: 'checking' });
    expect(s.display.eyebrow).toBe('starting');
    expect(s.display.headlinePre).toBe('Checking the');
    expect(s.display.headlineKey).toBe('foundation.');
    expect(s.display.headlineTone).toBe('accent');
  });

  it('resolving status produces "setting up" eyebrow + accent in-order key', () => {
    const s = makeState({ status: 'resolving' });
    expect(s.display.eyebrow).toBe('setting up');
    expect(s.display.headlineKey).toBe('in order.');
    expect(s.display.headlineTone).toBe('accent');
  });

  it('needs-action status produces "needs your hand" + step. key', () => {
    const s = makeState({ status: 'needs-action' });
    expect(s.display.eyebrow).toBe('needs your hand');
    expect(s.display.headlineKey).toBe('step.');
    expect(s.display.headlineTone).toBe('accent');
  });

  it('ok status produces "ready" + holds. key with success tone', () => {
    const s = makeState({ status: 'ok' });
    expect(s.display.eyebrow).toBe('ready');
    expect(s.display.headlineKey).toBe('holds.');
    expect(s.display.headlineTone).toBe('success');
  });

  it('heroTitle uses installingVerb when status="resolving"', () => {
    const s = makeState({
      status: 'resolving',
      readyIds: ['homebrew', 'postgres'],
      installingId: 'ollama',
      installingVerbs: { ollama: 'installing' },
    });
    expect(s.display.heroTitle).toBe('Installing · 2/6');
  });

  it('heroTitle capitalizes whatever verb the wire provides', () => {
    const s = makeState({
      status: 'resolving',
      readyIds: ['homebrew', 'postgres', 'ollama', 'sensei'],
      installingId: 'database',
      installingVerbs: { database: 'creating' },
    });
    expect(s.display.heroTitle).toBe('Creating · 4/6');
  });

  it('heroTitle is "The foundation holds" when status="ok"', () => {
    const s = makeState({ status: 'ok' });
    expect(s.display.heroTitle).toBe('The foundation holds');
  });

  it('heroTitle is "Needs your hand" when status="needs-action"', () => {
    const s = makeState({ status: 'needs-action' });
    expect(s.display.heroTitle).toBe('Needs your hand');
  });
});

describe('HealthState — retry()', () => {
  it('retry(id) triggers a check for the given gate via transport', () => {
    const calls: string[] = [];
    const s = makeState({ transport: { retry: (id: string) => calls.push(id) } });
    s.retry('ollama');
    expect(calls).toEqual(['ollama']);
  });
});
```

Add a `makeState` helper at the top of the file (or extend the existing one — read the file first to see):

```ts
import { HealthState } from './health-state.svelte.js';
import type { Component, HealthStatus } from './health-types.js';

interface MakeStateOpts {
  status?: HealthStatus;
  readyIds?: string[];
  installingId?: string;
  failedId?: string;
  installingVerbs?: Record<string, string>;
  transport?: { retry?: (id: string) => void };
}

function makeComponent(id: string, label: string, opts: MakeStateOpts): Component {
  let status: Component['status'] = 'pending';
  if (opts.readyIds?.includes(id)) status = 'ready';
  if (opts.installingId === id) status = 'installing';
  if (opts.failedId === id) status = 'failed';
  return {
    id: id as Component['id'],
    label,
    detail: null,
    note: null,
    status,
    version: null,
    installingVerb: opts.installingVerbs?.[id] ?? 'installing',
    description: `${label} description`,
  };
}

function makeState(opts: MakeStateOpts): HealthState {
  const s = new HealthState(opts.transport ?? {});
  s.status = opts.status ?? 'checking';
  s.packageManager = makeComponent('homebrew', 'Homebrew', opts);
  s.components = [
    makeComponent('postgres', 'PostgreSQL', opts),
    makeComponent('ollama',   'Ollama',     opts),
    makeComponent('sensei',   'Sensei',     opts),
    makeComponent('database', 'Database',   opts),
    makeComponent('daemon',   'Daemon',     opts),
  ];
  return s;
}
```

Adjust the constructor call to match the actual `HealthState` constructor signature you read in Step 1.

- [ ] **Step 3: Run tests to verify they fail**

Run: `cd sensei/app && bun run test -- health-state`
Expected: FAIL on every new test (`s.gates is not a function`, etc.).

- [ ] **Step 4: Add the new getters and `retry()` to `HealthState`**

Modify `sensei/app/src/lib/health-state.svelte.ts` — add these inside the `HealthState` class:

```ts
// ── Collection derivations ─────────────────────────────────────
get gates(): Component[] {
  return [this.packageManager, ...this.components];
}

get total(): number {
  return this.gates.length;
}

get readyCount(): number {
  return this.gates.filter((g) => g.status === 'ready').length;
}

get activeLabel(): string {
  return this.gates.find((g) => g.status === 'installing' || g.status === 'checking')?.label ?? '';
}

get firstBlockedIdx(): number {
  return this.gates.findIndex((g) => g.status === 'failed');
}

// ── Display copy per HealthStatus ──────────────────────────────
get display(): {
  eyebrow:      string;
  headlinePre:  string;
  headlineKey:  string;
  headlineTone: 'success' | 'accent' | 'ink-mute';
  subCopy:      string;
  heroTitle:    string;
} {
  switch (this.status) {
    case 'checking':
      return {
        eyebrow:      'starting',
        headlinePre:  'Checking the',
        headlineKey:  'foundation.',
        headlineTone: 'accent',
        subCopy:      'A quick health check before opening the observatory.',
        heroTitle:    this.#composeHeroTitle('Checking components'),
      };
    case 'resolving':
      return {
        eyebrow:      'setting up',
        headlinePre:  'Putting the room',
        headlineKey:  'in order.',
        headlineTone: 'accent',
        subCopy:      'Running brew bundle with the manifest from sensei-hq/homebrew-tap. No input needed.',
        heroTitle:    this.#composeHeroTitle('Checking components'),
      };
    case 'needs-action':
      return {
        eyebrow:      'needs your hand',
        headlinePre:  'One last',
        headlineKey:  'step.',
        headlineTone: 'accent',
        subCopy:      "Homebrew isn't here yet. Run the script — it installs Homebrew, then everything else.",
        heroTitle:    'Needs your hand',
      };
    case 'ok':
      return {
        eyebrow:      'ready',
        headlinePre:  'The foundation',
        headlineKey:  'holds.',
        headlineTone: 'success',
        subCopy:      'Homebrew, Postgres, Ollama, sensei components, database, and the daemon are all present. Opening the observatory.',
        heroTitle:    'The foundation holds',
      };
  }
}

#composeHeroTitle(fallback: string): string {
  const active = this.gates.find((g) => g.status === 'installing' || g.status === 'checking');
  if (!active) return fallback;
  const verb = active.installingVerb;
  const capitalized = verb.charAt(0).toUpperCase() + verb.slice(1);
  return `${capitalized} · ${this.readyCount}/${this.total}`;
}

// ── Action ─────────────────────────────────────────────────────
retry(id: ComponentId | PackageManagerId): void {
  this.transport.retry?.(id);
}
```

Wire `retry` into the transport interface if it's not there yet. Read `health-transport.ts` to see the existing transport contract — if `retry` doesn't exist as an injectable method, add an optional `retry?: (id: string) => void` to whatever transport-handle shape the constructor accepts.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cd sensei/app && bun run test -- health-state`
Expected: PASS (all existing + new tests green).

- [ ] **Step 6: Run full Vitest + lint + check**

Run:
```
cd sensei/app && bun run test && bun run lint && bun run check
```
Expected: zero failures.

- [ ] **Step 7: Commit**

```bash
cd sensei
git add app/src/lib/health-state.svelte.ts \
        app/src/lib/health-state.spec.svelte.ts \
        app/src/lib/health-transport.ts        # only if you modified it for retry
git commit -m "feat(app): add display + derivations + retry to HealthState"
```

---

## Task 9: Rewrite `Header.svelte` to use Wordmark + state.display

Replace the inline `先生 Sensei` markup with `<Wordmark>` and pull all status-keyed copy from `state.display`. Header becomes a pure template.

**Files:**
- Modify: `sensei/app/src/routes/(health)/health/Header.svelte`
- Modify: `sensei/app/src/routes/(health)/health/HealthView.svelte` (pass `state` down)

- [ ] **Step 1: Update Header.svelte signature + body**

Replace the contents of `sensei/app/src/routes/(health)/health/Header.svelte` with:

```svelte
<script lang="ts">
  import type { HealthState } from '$lib/health-state.svelte.js';
  import { Wordmark, Eyebrow } from '$lib/components';

  interface Props { state: HealthState; }
  let { state }: Props = $props();

  const headlineKeyTone = $derived(
    state.display.headlineTone === 'success' ? 'text-success'
    : state.display.headlineTone === 'accent' ? 'text-accent'
    :                                            'text-ink-mute',
  );
</script>

<header class="flex flex-col gap-4 mb-6">
  <Wordmark size="md" />

  <div class="flex flex-col gap-2">
    <Eyebrow>{state.display.eyebrow}</Eyebrow>

    <h1 class="font-display text-3xl font-light leading-tight tracking-tight text-ink">
      {state.display.headlinePre}
      <span class={headlineKeyTone}>{state.display.headlineKey}</span>
    </h1>

    <p data-sub class="text-sm text-ink-soft leading-relaxed max-w-prose">
      {state.display.subCopy}
    </p>
  </div>
</header>
```

- [ ] **Step 2: Update HealthView.svelte to pass `state` to Header**

Read `sensei/app/src/routes/(health)/health/HealthView.svelte`. Locate the `<Header status={state.status} />` call and change it to `<Header {state} />`. Don't touch the rest of HealthView yet — that's Task 11.

- [ ] **Step 3: Run lint + check**

Run: `cd sensei/app && bun run check && bun run lint`
Expected: zero errors.

- [ ] **Step 4: Run e2e behaviour test**

Run: `cd sensei/app && bun run test:e2e -- --grep boot-flow`
Expected: PASS (no behaviour regression).

- [ ] **Step 5: Manual visual check**

Run: `cd sensei && make app-dev` and navigate to `/health` in each `HealthStatus`. Confirm in light + dark:
- `先生 Sensei` displays via `Wordmark` (vermillion kanji, ink word).
- Eyebrow shows the right text per status (`starting`, `setting up`, `needs your hand`, `ready`).
- Headline pattern matches: `"Checking the foundation."` / `"Putting the room in order."` / `"One last step."` / `"The foundation holds."` with the right tone on the key word.
- Sub copy updates per status.

- [ ] **Step 6: Commit**

```bash
cd sensei
git add app/src/routes/\(health\)/health/Header.svelte \
        app/src/routes/\(health\)/health/HealthView.svelte
git commit -m "refactor(app): Header reads state.display; uses Wordmark"
```

---

## Task 10: `Remedy.svelte` token migration + button label change

Pure class substitution + verify-button label change. Panel structure (kanji 手 header, intro, script, footer) unchanged per spec §6.

**Files:**
- Modify: `sensei/app/src/routes/(health)/health/Remedy.svelte`

- [ ] **Step 1: Apply class substitutions**

Open `sensei/app/src/routes/(health)/health/Remedy.svelte`. Apply these substitutions across the file:

| Old class                                                                                    | New class                          |
| -------------------------------------------------------------------------------------------- | ---------------------------------- |
| `bg-surface-z1`                                                                              | `bg-paper-soft`                    |
| `bg-surface-z3`                                                                              | `bg-paper-mute`                    |
| `border-surface-z2`                                                                          | `border-paper-edge`                |
| `border-primary-z5/30`                                                                       | `border-accent/30`                 |
| `text-surface-z9`                                                                            | `text-ink`                         |
| `text-surface-z7`                                                                            | `text-ink-mute`                    |
| Inline `style="color: oklch(var(--color-primary-z5) / 1); border-color: oklch(var(--color-primary-z5) / 0.4);"` (verify button) | DELETE; add classes `text-accent border-accent/40` |

- [ ] **Step 2: Change the verify-button label**

In the same file, find the verify button:
```svelte
<button data-action="verify" ...>
  I've run it · verify
</button>
```

Change the visible label to:
```svelte
  I've run it · re-check
```

- [ ] **Step 3: Run rg to confirm no remnants**

Run:
```
rg "z[0-9]+|oklch\(var" sensei/app/src/routes/\(health\)/health/Remedy.svelte
```
Expected: empty output.

- [ ] **Step 4: Run lint + check + e2e**

Run:
```
cd sensei/app && bun run check && bun run lint && bun run test:e2e -- --grep boot-flow
```
Expected: PASS.

- [ ] **Step 5: Manual visual check (needs-action)**

`make app-dev`, drive into `needs-action`, confirm:
- Panel border has vermillion tint at 30% opacity (was hardcoded inline).
- Verify button reads "I've run it · re-check", styled with vermillion text + vermillion 40% border.
- Light + dark both look right.

- [ ] **Step 6: Commit**

```bash
cd sensei
git add app/src/routes/\(health\)/health/Remedy.svelte
git commit -m "refactor(app): Remedy uses named tokens; verify button label"
```

---

## Task 11: Rebuild `HealthView.svelte` against bootstrap-splash; delete Hero + Ledger

Compose the two-column layout from primitives + state. Delete `Hero.svelte` and `Ledger.svelte` (their content is now `KanjiHeader + GateRow × gates`).

**Files:**
- Modify: `sensei/app/src/routes/(health)/health/HealthView.svelte`
- Modify: `sensei/app/src/routes/(health)/health/+page.svelte` (pass state, drop intermediate)
- Delete: `sensei/app/src/routes/(health)/health/Hero.svelte`
- Delete: `sensei/app/src/routes/(health)/health/Ledger.svelte`

- [ ] **Step 1: Define the kanji numeral map**

The mockup uses 一二三四五六 for the 6 gates in order. Constant goes inside `HealthView.svelte` (not state — it's a presentational lookup keyed on gate index):

```ts
const NUMERALS = ['一', '二', '三', '四', '五', '六'] as const;
```

- [ ] **Step 2: Rewrite HealthView.svelte**

Replace `sensei/app/src/routes/(health)/health/HealthView.svelte` with:

```svelte
<script lang="ts">
  import type { HealthState } from '$lib/health-state.svelte.js';
  import { KanjiHeader, StatusDisc } from '$lib/components';
  import Header from './Header.svelte';
  import Footer from './Footer.svelte';
  import Remedy from './Remedy.svelte';
  import GateRow from './GateRow.svelte';

  interface Props {
    state: HealthState;
    onEnter?: () => void;
    onVerify?: () => void;
  }
  let { state, onEnter, onVerify }: Props = $props();

  const NUMERALS = ['一', '二', '三', '四', '五', '六'] as const;

  const showChecks = $derived(state.status !== 'ok');

  // Overall status for the right-column hero disc.
  // Maps HealthStatus → a ComponentStatus the disc renders.
  const heroDiscStatus = $derived.by(() => {
    if (state.status === 'ok') return 'ready' as const;
    if (state.status === 'needs-action') return 'failed' as const;
    return 'checking' as const;
  });
</script>

<div class="flex-1 min-h-0 overflow-y-auto px-8 py-10">
  <div
    class="w-full mx-auto grid {showChecks ? 'lg:grid-cols-[1fr_1px_1.05fr]' : 'grid-cols-1'} gap-x-7 gap-y-8 min-h-full"
    style="max-width: {showChecks ? '1000px' : '720px'};"
  >
    <!-- Left column · identity, headline, remedy, footer -->
    <div class="flex flex-col min-w-0">
      <Header {state} />

      {#if state.needsAction && state.remedy}
        <Remedy remedy={state.remedy} {onVerify} />
      {/if}

      {#if state.status === 'ok'}
        <div class="mt-5 flex items-center gap-2.5 text-xs text-ink-soft">
          <div class="h-[2px] w-20 bg-success rounded-sm" style="animation: tickle 2.4s ease-in-out infinite;"></div>
          <span class="font-mono tracking-tight">opening…</span>
        </div>
      {/if}

      <div class="mt-auto pt-8">
        <Footer version={state.version} platform={state.platform} />
      </div>
    </div>

    {#if showChecks}
      <!-- Divider -->
      <div class="bg-paper-edge"></div>

      <!-- Right column · hero, ledger, continue -->
      <div class="flex flex-col gap-5 min-w-0">
        <KanjiHeader kanji="支" eyebrow="foundation">
          {#snippet title()}{state.display.heroTitle}{/snippet}
          {#snippet right()}<StatusDisc status={heroDiscStatus} size={32} />{/snippet}
        </KanjiHeader>

        <div class="flex-1 min-h-0 flex flex-col border-t border-paper-edge">
          {#each state.gates as gate, i (gate.id)}
            <GateRow {gate} numeral={NUMERALS[i]} />
          {/each}
        </div>

        {#if state.isOk}
          <div class="flex justify-end pt-2">
            <button data-action="continue" class="btn-solid" onclick={onEnter}>Continue →</button>
          </div>
        {/if}
      </div>
    {:else}
      <!-- All-green: logo watermark anchored mid-right -->
      <div
        aria-hidden="true"
        class="absolute right-7 top-1/2 -translate-y-1/2 w-[260px] h-[260px] bg-ink opacity-10 pointer-events-none select-none"
        style="
          -webkit-mask-image: url('/sensei.svg');
                  mask-image: url('/sensei.svg');
          -webkit-mask-size: contain;
                  mask-size: contain;
          -webkit-mask-repeat: no-repeat;
                  mask-repeat: no-repeat;
          -webkit-mask-position: center;
                  mask-position: center;
        "
      ></div>
    {/if}
  </div>
</div>

<style>
  @keyframes tickle {
    0%, 100% { transform: scaleX(0.92); opacity: 0.6; }
    50%      { transform: scaleX(1);    opacity: 1; }
  }
</style>
```

The `<style>` block here contains **only** the `tickle` keyframe animation — no color, no spacing. That's the allowed exception per guideline §3.4.

- [ ] **Step 3: Verify `Footer.svelte` tokens**

Open `sensei/app/src/routes/(health)/health/Footer.svelte`. Apply substitutions:

| Old class             | New class             |
| --------------------- | --------------------- |
| `text-surface-z6`     | `text-ink-mute`       |
| `bg-surface-z6`       | `bg-ink-mute`         |

Replace `mono` (if used as a custom class) with `font-mono`. Run `rg "z[0-9]+|oklch\(var" sensei/app/src/routes/\(health\)/health/Footer.svelte` — expect empty.

- [ ] **Step 4: Delete Hero + Ledger**

```bash
cd sensei/app
rm src/routes/\(health\)/health/Hero.svelte
rm src/routes/\(health\)/health/Ledger.svelte
```

- [ ] **Step 5: Update +page.svelte if it imports the deleted files**

Read `sensei/app/src/routes/(health)/health/+page.svelte`. If it imports `Hero` or `Ledger` directly, remove those imports. It should currently just render `<HealthView ... />` — verify and adjust.

- [ ] **Step 6: Verify build + tests**

Run:
```
cd sensei/app && bun run check && bun run lint && bun run test && bun run test:e2e -- --grep boot-flow
```
Expected: all green. If a test references the deleted files, update or delete that test (state spec is the source of truth now; component snapshots for Hero/Ledger no longer apply).

- [ ] **Step 7: Run rg sweep — no z-scale or oklch in /health**

Run:
```
rg "z[0-9]+|oklch\(var" sensei/app/src/routes/\(health\)/health/
```
Expected: empty. If anything remains, fix it before committing.

- [ ] **Step 8: Manual visual check — all four states, both modes**

`make app-dev`. Drive each `HealthStatus` (via the test seam on `healthState` if available, or by driving the actual daemon). For each of `checking`, `resolving`, `needs-action`, `ok` × `light`, `dark`:
- Compare against `docs/mockups/Sensei/lib/bootstrap-splash.jsx` rendered in the design canvas.
- Confirm two-column layout when `status !== 'ok'`.
- Confirm `Ledger` rows show kanji numerals 一-六, zen italic descriptions, mono uppercase status labels with 20px discs.
- Confirm `ok` state hides the right column and shows the logo watermark + "opening…" tickle.
- Confirm Hero KanjiHeader title reads `"Installing · 3/6"` (or whatever verb the wire ships) when resolving, not a hardcoded "Installing".

- [ ] **Step 9: Commit**

```bash
cd sensei
git add app/src/routes/\(health\)/health/HealthView.svelte \
        app/src/routes/\(health\)/health/Footer.svelte \
        app/src/routes/\(health\)/health/+page.svelte
git rm app/src/routes/\(health\)/health/Hero.svelte \
       app/src/routes/\(health\)/health/Ledger.svelte
git commit -m "feat(app): rebuild HealthView against bootstrap-splash mockup"
```

---

## Task 12: `(health)/+layout.svelte` token swap + skin re-alignment + final verification

Two remaining items: migrate `(health)` group layout off `bg-surface-z0`, then do the skin re-alignment that the spec calls for (now safe because nothing in `/health` uses `text-primary-z*` anymore).

**Files:**
- Modify: `sensei/app/src/routes/(health)/+layout.svelte`
- Modify: `sensei/app/rokkit.config.js`

- [ ] **Step 1: Update (health)/+layout.svelte**

Open `sensei/app/src/routes/(health)/+layout.svelte`. Replace:

```svelte
<div class="w-full h-screen flex flex-col bg-surface-z0 overflow-hidden">
```

with:

```svelte
<div class="w-full h-screen flex flex-col bg-paper overflow-hidden">
```

- [ ] **Step 2: rg check — no z-scale in /(health) at all**

Run:
```
rg "z[0-9]+|oklch\(var" sensei/app/src/routes/\(health\)/
```
Expected: empty across the entire `(health)` route group.

- [ ] **Step 3: Skin re-alignment in rokkit.config.js**

Open `sensei/app/rokkit.config.js`. In the `skin:` block:

```js
skin: {
  surface:   { light: "kami", dark: "sumi" },
  ink:       { light: "kami", dark: "sumi" },
  primary:   "shu",       // ← change this
  secondary: "murasaki",
  accent:    "fuji",      // ← change this
  success:   "hisui",
  warning:   "kohaku",
  danger:    "beni",
  error:     "beni",
  info:      "ai",
},
```

Change to:

```js
skin: {
  surface:   { light: "kami", dark: "sumi" },
  ink:       { light: "kami", dark: "sumi" },
  primary:   { light: "kami", dark: "sumi" },   // primary CTA = ink-on-paper (overrides set ink color)
  secondary: "murasaki",
  accent:    "shu",                              // vermillion accent
  success:   "hisui",
  warning:   "kohaku",
  danger:    "beni",
  error:     "beni",
  info:      "ai",
},
```

- [ ] **Step 4: Remove now-redundant `accent-soft` override; keep `accent` for dark-mode shift**

With the skin's `accent: 'shu'`, the canonical `accent-soft` default (shade 100
of accent role) resolves to `shu.100` in both modes — same as the override.
**Delete** this line from `overrides:`:

```js
  "accent-soft": { light: "shu.100", dark: "shu.200" },
```

**Keep** the `accent` override:

```js
  accent: { light: "shu.500", dark: "shu.400" },
```

The canonical default would resolve to `shu.500` in both modes; we want
`shu.400` in dark mode for legibility, matching the dark-mode lightening that
`success`, `warning`, `danger`, `info` overrides already apply.

All other overrides (paper, ink, primary, on-primary, status dark shifts)
stay as written in Task 1.

- [ ] **Step 5: Run check + lint + test + e2e**

```
cd sensei/app && bun run check && bun run lint && bun run test && bun run test:e2e -- --grep boot-flow
```
Expected: all green.

- [ ] **Step 6: Manual visual confirm — nothing changed**

`make app-dev`, drive each `HealthStatus`, light + dark. The visual should be **identical** to the end of Task 11. The skin re-alignment is invisible from `/health`'s point of view because:
- `/health` no longer uses `text-primary-z*` (migrated to `text-accent`).
- `text-accent` previously resolved to shu via the override; now resolves to shu via the skin. Same color.
- `bg-primary` (named token) was already overridden to ink; now ink via the skin's primary palette + override. Same color.

If anything looks broken, the most likely cause is residual `text-primary-z*` somewhere we missed — re-run the rg check and migrate it.

- [ ] **Step 7: Acceptance criteria sweep**

Run each acceptance command from spec §Acceptance criteria:

```
rg "z[0-9]+|oklch\(var" sensei/app/src/routes/\(health\)/health/   # → empty
rg "<style>" sensei/app/src/routes/\(health\)/health/              # → only animations/geometry
cd sensei/app && bun run test                                      # → green
cd sensei/app && bun run test:e2e -- --grep boot-flow              # → green
```

If any check fails, fix and re-run before committing.

- [ ] **Step 8: Commit + push**

```bash
cd sensei
git add app/src/routes/\(health\)/+layout.svelte \
        app/rokkit.config.js
git commit -m "feat(app): skin re-alignment + (health) layout token swap"
git push origin develop
```

- [ ] **Step 9: Update the backlog**

Open `sensei/docs/backlog.md`. Locate the Phase 1 health-migration entry. Mark it complete with the date. If there's no entry, add a "Frontend migration — Phase 1 (`/health`) complete" line under the appropriate section with today's date and a one-line summary of what shipped (substitution table applied; 6 primitives extracted; skin re-aligned; visual matches bootstrap-splash mockup).

- [ ] **Step 10: Commit the backlog update**

```bash
cd sensei
git add docs/backlog.md
git commit -m "docs: mark Phase 1 health rokkit migration complete"
git push origin develop
```

---

## Out-of-scope reminders

Per spec §Out-of-scope:

- **Phase 2**: `/health/upgrade`, `/health/logs` migration. Same pattern.
- **Phase 3+**: setup wizard, observatory, project pages.
- **Promote primitives**: `Wordmark`, `KanjiHeader`, `StatusDisc`, `Spinner` already in `$lib/components/`. `StatusIndicator`, `GateRow` stay in `(health)/health/` until a second consumer pulls them up.

## Open decisions deferred to implementation

Per spec §Open questions:

- **`paper-edge` dark value** — currently `sumi.100` (etched). The mockup CSS suggests softer (~`sumi.300`). During Task 11 manual visual check, decide whether to soften. If yes, edit Task 1's override block and amend the Task 1 commit (or add a follow-up commit).
- **All-green hold delay** — Task 11 currently relies on the existing instant `goto('/')` in `+page.svelte`. If the watermark + "opening…" treatment flashes too briefly to be visible, add `setTimeout(() => goto('/'), 600)` in `+page.svelte`. Decide during Task 11 manual visual check.
