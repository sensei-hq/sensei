# Desktop Redesign Phase 1 — Design System + Setup Wizard

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace UnoCSS with the zen/sumi design token system and rewrite the 6-step setup wizard as a 9-step wizard matching the Claude Design mockups.

**Architecture:** Remove UnoCSS + Rokkit, replace with CSS custom properties (light/dark). Bundle 3 font families locally. Split the monolithic setup page into 9 step components under `src/lib/setup/`. Mock daemon data where APIs don't exist yet.

**Tech Stack:** SvelteKit 2 + Svelte 5 (runes), Tauri 2, CSS custom properties (no utility framework), local WOFF2 fonts

---

## File Map

### Create
```
apps/desktop/src/lib/tokens.css              — zen/sumi design tokens (light + dark)
apps/desktop/src/lib/base.css                — reset + typography + shared utilities
apps/desktop/static/fonts/inter-*.woff2      — Inter font files (4 weights)
apps/desktop/static/fonts/fraunces-*.woff2   — Fraunces font files (4 weights)
apps/desktop/static/fonts/jetbrains-*.woff2  — JetBrains Mono (2 weights)
apps/desktop/src/lib/setup/types.ts          — wizard state types + mock data
apps/desktop/src/lib/setup/WizRail.svelte    — left rail stepper
apps/desktop/src/lib/setup/WizBottom.svelte  — bottom bar (progress + nav)
apps/desktop/src/lib/setup/WizWelcome.svelte — step 1
apps/desktop/src/lib/setup/WizComponents.svelte — step 2
apps/desktop/src/lib/setup/WizAssistants.svelte — step 3
apps/desktop/src/lib/setup/WizFolders.svelte — step 4
apps/desktop/src/lib/setup/WizScan.svelte    — step 5
apps/desktop/src/lib/setup/WizProjects.svelte — step 6
apps/desktop/src/lib/setup/WizLibraries.svelte — step 7
apps/desktop/src/lib/setup/WizRegistry.svelte — step 8
apps/desktop/src/lib/setup/WizDone.svelte    — step 9
apps/desktop/src/lib/setup/index.ts          — re-exports
apps/desktop/src/lib/Sparkline.svelte        — SVG sparkline component
apps/desktop/src/lib/TauriChrome.svelte      — window chrome bar
```

### Modify
```
apps/desktop/src/app.css                     — replace UnoCSS imports with tokens
apps/desktop/src/app.html                    — add font preloads
apps/desktop/vite.config.ts                  — remove UnoCSS plugin
apps/desktop/package.json                    — remove UnoCSS + Rokkit deps
apps/desktop/src/routes/+layout.svelte       — remove accent-color probe, use tokens
apps/desktop/src/routes/setup/+page.svelte   — rewrite: thin shell dispatching to step components
```

### Delete
```
apps/desktop/uno.config.js
apps/desktop/rokkit.config.js
```

---

## Task 1: Bundle Fonts Locally

**Files:**
- Create: `apps/desktop/static/fonts/` (directory with WOFF2 files)
- Create: `apps/desktop/src/lib/fonts.css`

- [ ] **Step 1: Download font files**

```bash
cd apps/desktop
mkdir -p static/fonts

# Inter (400, 500, 600 — variable weight subset)
curl -L "https://fonts.gstatic.com/s/inter/v18/UcCO3FwrK3iLTeHuS_nVMrMxCp50SjIw2boKoduKmMEVuLyfMZg.woff2" -o static/fonts/inter-latin.woff2

# Fraunces (400, 500, 600 — variable optical size + weight)  
curl -L "https://fonts.gstatic.com/s/fraunces/v32/6NUh8FyLNQOQZAnv9bYEvDiIdE9Ea92uemAk_WBq8U_9v0c2Wa0K7iN7hzFUPJH58nk.woff2" -o static/fonts/fraunces-latin.woff2

# JetBrains Mono (400, 500)
curl -L "https://fonts.gstatic.com/s/jetbrainsmono/v20/tDbY2o-flEEny0FZhsfKu5WU4zr3E_BX0PnT8RD8yKxjPVmUsaaDhw.woff2" -o static/fonts/jetbrains-mono-latin.woff2
```

- [ ] **Step 2: Create font-face declarations**

Write `apps/desktop/src/lib/fonts.css`:

```css
/* Bundled fonts — no network dependency */
@font-face {
  font-family: 'Inter';
  font-style: normal;
  font-weight: 300 600;
  font-display: swap;
  src: url('/fonts/inter-latin.woff2') format('woff2');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}

@font-face {
  font-family: 'Fraunces';
  font-style: normal;
  font-weight: 300 600;
  font-display: swap;
  src: url('/fonts/fraunces-latin.woff2') format('woff2');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}

@font-face {
  font-family: 'JetBrains Mono';
  font-style: normal;
  font-weight: 400 500;
  font-display: swap;
  src: url('/fonts/jetbrains-mono-latin.woff2') format('woff2');
  unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD;
}
```

- [ ] **Step 3: Verify fonts load**

```bash
ls -la static/fonts/
# Should show 3 woff2 files, each 50-150KB
```

- [ ] **Step 4: Commit**

```bash
git add static/fonts/ src/lib/fonts.css
git commit -m "feat(desktop): bundle Inter, Fraunces, JetBrains Mono locally"
```

---

## Task 2: Create Zen/Sumi Design Tokens

**Files:**
- Create: `apps/desktop/src/lib/tokens.css`

- [ ] **Step 1: Write token file**

Write `apps/desktop/src/lib/tokens.css` — copy from the design artifact at `docs/design/02-desktop/setup/lib/tokens.css` but adapted for our app:

```css
/* Sensei — zen/sumi design tokens */
@import './fonts.css';

:root {
  /* Washi paper — light mode */
  --paper: oklch(0.975 0.008 85);
  --paper-2: oklch(0.955 0.010 85);
  --paper-3: oklch(0.92 0.012 85);
  --paper-edge: oklch(0.88 0.015 85);

  --sumi: oklch(0.22 0.012 50);
  --sumi-2: oklch(0.38 0.012 50);
  --sumi-3: oklch(0.58 0.010 50);
  --sumi-4: oklch(0.75 0.008 50);

  --shu: oklch(0.58 0.15 35);
  --shu-soft: oklch(0.58 0.15 35 / 0.12);
  --jade: oklch(0.62 0.08 160);
  --jade-soft: oklch(0.62 0.08 160 / 0.14);
  --amber: oklch(0.72 0.12 75);
  --amber-soft: oklch(0.72 0.12 75 / 0.15);

  --font-display: 'Fraunces', 'Iowan Old Style', Georgia, serif;
  --font-ui: 'Inter', system-ui, -apple-system, sans-serif;
  --font-mono: 'JetBrains Mono', 'SF Mono', Menlo, monospace;

  --radius: 6px;
  --radius-lg: 10px;
  --hairline: 1px solid var(--paper-edge);
  --ink-line: 1px solid oklch(0.22 0.012 50 / 0.12);
}

[data-mode="dark"] {
  --paper: oklch(0.17 0.010 50);
  --paper-2: oklch(0.21 0.012 50);
  --paper-3: oklch(0.25 0.012 50);
  --paper-edge: oklch(0.32 0.012 50);

  --sumi: oklch(0.94 0.008 85);
  --sumi-2: oklch(0.78 0.008 85);
  --sumi-3: oklch(0.60 0.010 85);
  --sumi-4: oklch(0.42 0.012 85);

  --shu: oklch(0.70 0.15 35);
  --shu-soft: oklch(0.70 0.15 35 / 0.18);
  --jade: oklch(0.72 0.09 160);
  --jade-soft: oklch(0.72 0.09 160 / 0.18);
  --amber: oklch(0.78 0.12 75);
  --amber-soft: oklch(0.78 0.12 75 / 0.20);

  --ink-line: 1px solid oklch(0.94 0.008 85 / 0.12);
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/tokens.css
git commit -m "feat(desktop): add zen/sumi design tokens"
```

---

## Task 3: Create Base CSS (Reset + Typography + Utilities)

**Files:**
- Create: `apps/desktop/src/lib/base.css`

- [ ] **Step 1: Write base styles**

Write `apps/desktop/src/lib/base.css`:

```css
/* Reset + typography + shared utilities */
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

html, body { height: 100%; overflow: hidden; }

body {
  font-family: var(--font-ui);
  font-size: 13px;
  line-height: 1.5;
  color: var(--sumi);
  background: var(--paper);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

button { font-family: inherit; color: inherit; background: none; border: none; cursor: pointer; padding: 0; }
input, textarea, select { font-family: inherit; color: inherit; }
a { color: inherit; text-decoration: none; }

/* Typography classes */
.display { font-family: var(--font-display); font-feature-settings: "ss01"; letter-spacing: -0.02em; }
.mono { font-family: var(--font-mono); font-variant-numeric: tabular-nums; letter-spacing: -0.01em; }
.kanji { font-family: "Yu Mincho", "Hiragino Mincho ProN", "Songti SC", serif; }

/* Hairline separator */
.hairline { background: var(--paper-edge); height: 1px; border: none; }

/* Ink dot */
.ink-dot { display: inline-block; width: 6px; height: 6px; border-radius: 50%; background: var(--shu); }

/* Tauri window chrome */
.drag-region { -webkit-app-region: drag; }
.no-drag { -webkit-app-region: no-drag; }

/* Scrollbars */
::-webkit-scrollbar { width: 6px; height: 6px; }
::-webkit-scrollbar-thumb { background: transparent; border-radius: 3px; }
:hover::-webkit-scrollbar-thumb { background: oklch(0.5 0.01 50 / 0.2); }

/* Focus ring */
button:focus-visible { outline: 1.5px solid var(--shu); outline-offset: 2px; border-radius: 3px; }

/* Selection */
::selection { background: var(--shu-soft); }
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/base.css
git commit -m "feat(desktop): add base CSS reset and typography"
```

---

## Task 4: Remove UnoCSS + Rokkit, Wire New Tokens

**Files:**
- Modify: `apps/desktop/vite.config.ts`
- Modify: `apps/desktop/src/app.css`
- Modify: `apps/desktop/src/app.html`
- Modify: `apps/desktop/src/routes/+layout.svelte`
- Delete: `apps/desktop/uno.config.js`
- Delete: `apps/desktop/rokkit.config.js`

- [ ] **Step 1: Remove UnoCSS from vite config**

Edit `apps/desktop/vite.config.ts` — remove UnoCSS import and plugin:

```typescript
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
```

- [ ] **Step 2: Replace app.css**

Write `apps/desktop/src/app.css`:

```css
@import '$lib/tokens.css';
@import '$lib/base.css';
```

- [ ] **Step 3: Add font preloads to app.html**

Edit `apps/desktop/src/app.html` — add preloads in `<head>`:

```html
<link rel="preload" href="/fonts/inter-latin.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="/fonts/fraunces-latin.woff2" as="font" type="font/woff2" crossorigin>
```

- [ ] **Step 4: Simplify root layout**

Rewrite `apps/desktop/src/routes/+layout.svelte` to use the token system instead of the UnoCSS accent probe:

```svelte
<script lang="ts">
  let { children } = $props();

  // Detect system color scheme
  if (typeof window !== 'undefined') {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = (dark: boolean) => document.body.dataset.mode = dark ? 'dark' : 'light';
    apply(mq.matches);
    mq.addEventListener('change', (e) => apply(e.matches));
  }
</script>

{@render children()}
```

- [ ] **Step 5: Delete old config files**

```bash
rm uno.config.js rokkit.config.js
```

- [ ] **Step 6: Remove UnoCSS + Rokkit packages**

```bash
bun remove unocss @unocss/vite @unocss/reset @unocss/extractor-svelte @rokkit/unocss @rokkit/themes @rokkit/ui
```

- [ ] **Step 7: Verify app builds**

```bash
bun run build 2>&1 | tail -5
```

Note: existing pages will look broken (no utility classes). That's expected — we're rewriting them.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "refactor(desktop): remove UnoCSS + Rokkit, wire zen/sumi tokens"
```

---

## Task 5: Shared UI Components

**Files:**
- Create: `apps/desktop/src/lib/TauriChrome.svelte`
- Create: `apps/desktop/src/lib/Sparkline.svelte`

- [ ] **Step 1: Create TauriChrome component**

Write `apps/desktop/src/lib/TauriChrome.svelte`:

```svelte
<script lang="ts">
  let { title = 'Sensei 先生' }: { title?: string } = $props();
</script>

<header class="chrome drag-region">
  <div class="traffic no-drag">
    <span class="dot close"></span>
    <span class="dot minimize"></span>
    <span class="dot zoom"></span>
  </div>
  <span class="title">{title}</span>
</header>

<style>
  .chrome {
    height: 38px;
    display: flex;
    align-items: center;
    padding: 0 14px;
    border-bottom: var(--hairline);
    background: var(--paper);
    gap: 8px;
    flex-shrink: 0;
  }
  .traffic { display: flex; gap: 7px; }
  .dot {
    width: 12px; height: 12px; border-radius: 50%;
    display: block; background: var(--paper-edge);
  }
  .dot.close { background: oklch(0.72 0.14 28); }
  .dot.minimize { background: oklch(0.82 0.13 85); }
  .dot.zoom { background: oklch(0.72 0.11 145); }
  .title {
    flex: 1; text-align: center;
    font-size: 12px; color: var(--sumi-3);
    letter-spacing: 0.02em;
  }
</style>
```

- [ ] **Step 2: Create Sparkline component**

Write `apps/desktop/src/lib/Sparkline.svelte`:

```svelte
<script lang="ts">
  let {
    data,
    width = 80,
    height = 22,
    color = 'currentColor',
  }: {
    data: number[];
    width?: number;
    height?: number;
    color?: string;
  } = $props();

  let path = $derived.by(() => {
    if (data.length < 2) return '';
    const min = Math.min(...data);
    const max = Math.max(...data);
    const range = max - min || 1;
    const pad = 1.5;
    const step = (width - pad * 2) / (data.length - 1);
    return data
      .map((v, i) => {
        const x = pad + i * step;
        const y = pad + (height - pad * 2) * (1 - (v - min) / range);
        return `${i ? 'L' : 'M'}${x.toFixed(1)} ${y.toFixed(1)}`;
      })
      .join(' ');
  });

  let endDot = $derived.by(() => {
    if (data.length < 2) return null;
    const min = Math.min(...data);
    const max = Math.max(...data);
    const range = max - min || 1;
    const pad = 1.5;
    const step = (width - pad * 2) / (data.length - 1);
    const last = data[data.length - 1];
    return {
      cx: pad + (data.length - 1) * step,
      cy: pad + (height - pad * 2) * (1 - (last - min) / range),
    };
  });
</script>

<svg {width} {height} style="color: {color}; display: block; overflow: visible;">
  <path d={path} fill="none" stroke="currentColor" stroke-width="1.25"
        stroke-linecap="round" stroke-linejoin="round" />
  {#if endDot}
    <circle cx={endDot.cx} cy={endDot.cy} r={2} fill="currentColor" />
  {/if}
</svg>
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/TauriChrome.svelte src/lib/Sparkline.svelte
git commit -m "feat(desktop): add TauriChrome and Sparkline components"
```

---

## Task 6: Setup Wizard Types + Mock Data

**Files:**
- Create: `apps/desktop/src/lib/setup/types.ts`

- [ ] **Step 1: Write types and mock data**

Write `apps/desktop/src/lib/setup/types.ts` — adapted from `docs/design/02-desktop/setup/lib/setup-data.js`:

```typescript
// Setup wizard types and mock data

export interface WizStage {
  id: string;
  n: string;     // kanji numeral
  title: string;
  sub: string;
}

export const WIZ_STAGES: WizStage[] = [
  { id: 'welcome',    n: '一', title: 'Welcome',       sub: 'a quiet observer of your work' },
  { id: 'components', n: '二', title: 'Components',    sub: 'installed automatically' },
  { id: 'assistants', n: '三', title: 'Assistants',    sub: 'plugins · skills · commands · logging' },
  { id: 'folders',    n: '四', title: 'Folders',       sub: 'where does your work live' },
  { id: 'scan',       n: '五', title: 'Scan',          sub: 'watching the worker' },
  { id: 'projects',   n: '六', title: 'Projects',      sub: 'one or more repos each' },
  { id: 'libraries',  n: '七', title: 'Libraries',     sub: 'what sensei should wrap' },
  { id: 'registry',   n: '八', title: 'MCP Registry',  sub: 'recommended for your stack' },
  { id: 'done',       n: '九', title: 'Enter',         sub: 'the observatory is ready' },
];

export interface ComponentStatus {
  id: string;
  name: string;
  version: string | null;
  status: 'missing' | 'installed' | 'stopped' | 'ready';
}

export interface AcpEntry {
  id: string;
  name: string;
  version: string | null;
  found: boolean;
  path: string | null;
}

export interface ScanFolder {
  id: string;
  path: string;
  note: string;
}

export interface ScanEvent {
  t: number;
  level: 'info' | 'discover' | 'queue' | 'process' | 'success';
  msg: string;
  parent?: string;
}

export interface DiscoveredProject {
  id: string;
  name: string;
  kanji: string;
  path: string;
  autoDetected: boolean;
  confidence: 'high' | 'medium' | 'low';
  repos: DiscoveredRepo[];
}

export interface DiscoveredRepo {
  id: string;
  name: string;
  path: string;
  files: number;
  lang: string;
  suggestedRole: string;
}

export interface DiscoveredLibrary {
  id: string;
  name: string;
  version: string;
  lang: string;
  usage: number;
  source: string;
  docs: 'indexed' | 'partial' | 'schema' | 'none';
  why: string;
}

export interface McpEntry {
  id: string;
  name: string;
  publisher: string;
  kind: 'data' | 'api' | 'devtool' | 'service';
  kanji: string;
  summary: string;
  trigger: string[];
  tools: number;
  verified: boolean;
  installed: boolean;
  recommended: boolean;
}

export interface RoleOption {
  id: string;
  label: string;
  kanji: string;
}

export const ROLES: RoleOption[] = [
  { id: 'backend',  label: 'Backend',  kanji: '後' },
  { id: 'frontend', label: 'Frontend', kanji: '前' },
  { id: 'library',  label: 'Library',  kanji: '書' },
  { id: 'docs',     label: 'Docs',     kanji: '記' },
  { id: 'infra',    label: 'Infra',    kanji: '基' },
];

// ─── Wizard state ───────────────────────────────────────────

export interface WizardState {
  components: ComponentStatus[];
  acps: Record<string, boolean>;
  folders: ScanFolder[];
  scanStarted: boolean;
  scanDone: boolean;
  scanTick: number;
  projects: (DiscoveredProject & { confirmed: boolean })[];
  roles: Record<string, string>;
  libraries: Record<string, boolean>;
  mcps: Record<string, boolean>;
}

export function createInitialState(
  acps: AcpEntry[],
  folders: ScanFolder[],
  projects: DiscoveredProject[],
  libraries: DiscoveredLibrary[],
  mcps: McpEntry[],
): WizardState {
  return {
    components: [],
    acps: Object.fromEntries(acps.map(a => [a.id, a.found])),
    folders: [...folders],
    scanStarted: false,
    scanDone: false,
    scanTick: 0,
    projects: projects.map(p => ({ ...p, confirmed: true })),
    roles: Object.fromEntries(
      projects.flatMap(p => p.repos.map(r => [r.id, r.suggestedRole]))
    ),
    libraries: Object.fromEntries(libraries.map(l => [l.id, true])),
    mcps: Object.fromEntries(mcps.map(m => [m.id, m.installed || m.recommended])),
  };
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/setup/types.ts
git commit -m "feat(desktop): add setup wizard types and state model"
```

---

## Task 7: WizRail + WizBottom (Navigation Shell)

**Files:**
- Create: `apps/desktop/src/lib/setup/WizRail.svelte`
- Create: `apps/desktop/src/lib/setup/WizBottom.svelte`

- [ ] **Step 1: Create WizRail**

Write `apps/desktop/src/lib/setup/WizRail.svelte` — the left sidebar stepper. Shows completed steps with checkmarks, current step highlighted, future steps dimmed. See screenshot `02-setup.png` for reference.

The component receives `stages`, `currentIndex`, and an `onNavigate` callback. Completed steps (index < current) show a checkmark. Current step shows kanji + title + subtitle in an active card. Future steps show kanji + title dimmed.

- [ ] **Step 2: Create WizBottom**

Write `apps/desktop/src/lib/setup/WizBottom.svelte` — the bottom bar. Shows step N / total, stage title, progress segments, Back button, Continue/Enter button. See screenshots for reference.

The component receives `stage`, `stageIndex`, `total`, `onBack`, `onNext`, `onDone`, and controls button labels (Continue vs Enter observatory on last step).

- [ ] **Step 3: Commit**

```bash
git add src/lib/setup/WizRail.svelte src/lib/setup/WizBottom.svelte
git commit -m "feat(desktop): add wizard rail and bottom navigation"
```

---

## Task 8: Step Components (Welcome through Done)

Each step is its own `.svelte` file. They receive `state` and an `update` function to patch state.

**Files:**
- Create: `apps/desktop/src/lib/setup/WizWelcome.svelte`
- Create: `apps/desktop/src/lib/setup/WizComponents.svelte`
- Create: `apps/desktop/src/lib/setup/WizAssistants.svelte`
- Create: `apps/desktop/src/lib/setup/WizFolders.svelte`
- Create: `apps/desktop/src/lib/setup/WizScan.svelte`
- Create: `apps/desktop/src/lib/setup/WizProjects.svelte`
- Create: `apps/desktop/src/lib/setup/WizLibraries.svelte`
- Create: `apps/desktop/src/lib/setup/WizRegistry.svelte`
- Create: `apps/desktop/src/lib/setup/WizDone.svelte`
- Create: `apps/desktop/src/lib/setup/index.ts`

Build each step component matching the design screenshots and JSX artifacts. Each step uses `<style>` blocks with the zen/sumi tokens — no utility classes.

**Implementation order:** Welcome → Done → Components → Assistants → Folders → Scan → Projects → Libraries → Registry

(Welcome and Done are simplest — pure display. Components and Assistants are card layouts. Folders is a form. Scan has the SSE animation. Projects, Libraries, Registry have interactive lists.)

- [ ] **Step 1: WizWelcome** — hero text + three pillars (screenshot 02-setup.png)
- [ ] **Step 2: WizDone** — 観 kanji + stats grid (screenshot 11-setup.png)
- [ ] **Step 3: WizComponents** — component status cards (screenshot 03-setup.png)
- [ ] **Step 4: WizAssistants** — ACP detection cards (screenshot 04-setup.png)
- [ ] **Step 5: WizFolders** — path input + folder list (screenshot 05-setup.png)
- [ ] **Step 6: WizScan** — live event log + stats (screenshot 06-07-setup.png)
- [ ] **Step 7: WizProjects** — project cards with role editing (screenshot 08-setup.png)
- [ ] **Step 8: WizLibraries** — library list + add button (screenshot 09-setup.png)
- [ ] **Step 9: WizRegistry** — MCP cards with stack chips (screenshot 10-setup.png)
- [ ] **Step 10: Create index.ts re-exports**
- [ ] **Step 11: Commit**

```bash
git add src/lib/setup/
git commit -m "feat(desktop): implement 9-step setup wizard components"
```

---

## Task 9: Rewrite Setup Page Shell

**Files:**
- Modify: `apps/desktop/src/routes/setup/+page.svelte`

- [ ] **Step 1: Rewrite setup page as thin dispatcher**

Replace the monolithic setup page with a shell that:
1. Manages stage index state
2. Loads mock data (for now) or real data from daemon
3. Renders TauriChrome + WizRail + current step + WizBottom

```svelte
<script lang="ts">
  import { goto } from '$app/navigation';
  import TauriChrome from '$lib/TauriChrome.svelte';
  import { WIZ_STAGES, createInitialState } from '$lib/setup/types';
  import WizRail from '$lib/setup/WizRail.svelte';
  import WizBottom from '$lib/setup/WizBottom.svelte';
  import WizWelcome from '$lib/setup/WizWelcome.svelte';
  import WizComponents from '$lib/setup/WizComponents.svelte';
  import WizAssistants from '$lib/setup/WizAssistants.svelte';
  import WizFolders from '$lib/setup/WizFolders.svelte';
  import WizScan from '$lib/setup/WizScan.svelte';
  import WizProjects from '$lib/setup/WizProjects.svelte';
  import WizLibraries from '$lib/setup/WizLibraries.svelte';
  import WizRegistry from '$lib/setup/WizRegistry.svelte';
  import WizDone from '$lib/setup/WizDone.svelte';

  let stageIdx = $state(0);
  let stage = $derived(WIZ_STAGES[stageIdx]);

  // TODO: replace with real daemon data
  let state = $state(createInitialState([], [], [], [], []));
  const update = (patch: Partial<typeof state>) => { state = { ...state, ...patch }; };

  const next = () => { stageIdx = Math.min(stageIdx + 1, WIZ_STAGES.length - 1); };
  const back = () => { stageIdx = Math.max(stageIdx - 1, 0); };
  const done = () => { goto('/overview'); };
  const exit = () => { goto('/overview'); };
</script>

<div class="wizard">
  <TauriChrome title="Sensei 先生 · setup" />
  <div class="body">
    <WizRail stages={WIZ_STAGES} currentIndex={stageIdx} onNavigate={(i) => stageIdx = i} onExit={exit} />
    <div class="main">
      <div class="content">
        {#if stage.id === 'welcome'}<WizWelcome />
        {:else if stage.id === 'components'}<WizComponents {state} {update} />
        {:else if stage.id === 'assistants'}<WizAssistants {state} {update} />
        {:else if stage.id === 'folders'}<WizFolders {state} {update} />
        {:else if stage.id === 'scan'}<WizScan {state} {update} />
        {:else if stage.id === 'projects'}<WizProjects {state} {update} />
        {:else if stage.id === 'libraries'}<WizLibraries {state} {update} />
        {:else if stage.id === 'registry'}<WizRegistry {state} {update} />
        {:else if stage.id === 'done'}<WizDone {state} />
        {/if}
      </div>
      <WizBottom {stage} stageIndex={stageIdx} total={WIZ_STAGES.length}
                 onBack={back} onNext={next} onDone={done} {state} />
    </div>
  </div>
</div>

<style>
  .wizard {
    width: 100%; height: 100vh;
    display: flex; flex-direction: column;
    background: var(--paper); overflow: hidden;
  }
  .body {
    flex: 1; display: grid;
    grid-template-columns: 260px 1fr;
    min-height: 0;
  }
  .main {
    display: flex; flex-direction: column; min-height: 0;
  }
  .content {
    flex: 1; overflow: auto;
    padding: 44px 64px 32px;
  }
</style>
```

- [ ] **Step 2: Verify setup page loads**

```bash
bun run dev
# Navigate to /setup in browser — should show the wizard shell
```

- [ ] **Step 3: Commit**

```bash
git add src/routes/setup/+page.svelte
git commit -m "refactor(desktop): rewrite setup page as thin wizard dispatcher"
```

---

## Task 10: Wire Real Data (daemon integration)

**Files:**
- Modify: `apps/desktop/src/routes/setup/+page.svelte`

- [ ] **Step 1: Load real ACP data from daemon**

In the setup page's `onMount`, call `senseiApi(port).detectAcps()` and `senseiApi(port).getHealth()` to populate the state with real component/ACP data. Fall back to mock data if daemon is unreachable.

- [ ] **Step 2: Wire folder scanning to daemon**

The Folders step should call `senseiApi(port).scanFolder(path)` when the user adds a folder. The Scan step subscribes to the SSE progress endpoint.

- [ ] **Step 3: Wire project creation**

The Projects step should call `senseiApi(port).createProject()` and `senseiApi(port).setRepoProject()` when the user confirms projects.

- [ ] **Step 4: Commit**

```bash
git add src/routes/setup/+page.svelte src/lib/setup/
git commit -m "feat(desktop): wire setup wizard to daemon API"
```

---

## Task 11: Create Daemon Gap Issues

- [ ] **Step 1: Create GitHub issues**

Create issues for daemon work needed to fully support the design:

1. **Stack detection** — detect languages/frameworks/runtimes/services from manifests
2. **Component health endpoint** — `GET /api/health/components` returning CLI + MCP + daemon status
3. **MCP registry** — global catalog + per-project install tracking
4. **Repo metadata API** — `GET /api/repos/{id}/metadata` exposing icon/links/summary
5. **Hero teaching system** — session analysis → most impactful insight
6. **Anti-pattern detection** — duplication, god nodes, dead code in patterns endpoint

```bash
gh issue create --title "feat(daemon): stack detection from manifests" --body "Detect languages, frameworks, runtimes, services from package.json/Cargo.toml/Dockerfile/docker-compose.yml. Needed for MCP Registry recommendations." --label "enhancement"

gh issue create --title "feat(daemon): component health endpoint" --body "GET /api/health/components — return status of sensei-cli, sensei-mcp, senseid (version, status: missing/installed/stopped/ready). Needed for setup wizard step 2." --label "enhancement"

gh issue create --title "feat(daemon): MCP registry" --body "Global MCP catalog (known MCPs with metadata) + per-project install tracking. GET /api/mcps, POST /api/mcps/install. Needed for setup wizard step 8 and MCP Registry page." --label "enhancement"

gh issue create --title "feat(daemon): repo metadata API" --body "GET /api/repos/{id}/metadata — return icon, external_links, summary from the metadata column added in the scanner. Desktop needs this for project cards and settings." --label "enhancement"

gh issue create --title "feat(daemon): hero teaching system" --body "Analyze sessions to identify the single most impactful insight. Correlate corrections with modules/patterns. Return via GET /api/teaching. Core observatory feature." --label "enhancement"

gh issue create --title "feat(daemon): anti-pattern detection" --body "Extend /api/patterns/{repo} to include anti-patterns: duplication, god nodes, monolithic code, dead code. Each with severity, locations, suggested fix pattern." --label "enhancement"
```

- [ ] **Step 2: Commit issue references to backlog doc**

---

## Verification

After all tasks complete:

1. `bun run build` succeeds with no errors
2. `bun run check` has no new type errors (pre-existing ones from non-setup pages are expected since they still use UnoCSS classes)
3. Setup wizard at `/setup` renders all 9 steps with correct visual design
4. Fonts load from local bundles (no network requests to Google Fonts)
5. Light/dark mode toggles correctly via system preference
6. The app (non-setup pages) still renders structurally even if colors are wrong — no crashes
