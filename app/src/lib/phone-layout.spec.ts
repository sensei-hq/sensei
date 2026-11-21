// Every screen lays out for a phone first (OpenSpec `update-phone-layout`, #420; frontend
// guidelines §1.7): unprefixed classes are the phone layout, `sm:`/`md:` adjust upward. This spec
// keeps it so: a NEW unprefixed layout wider than a phone fails here.
//
// Wider than a phone: `grid-cols-N` with N ≥ 3; an arbitrary grid with 4+ columns or a fixed
// column of 200px or more; a fixed width (`w-`/`min-w-[Npx]`) of 300px or more. A `max-*:` variant
// is desktop-first and never allowed. `class:` directives count as unprefixed.
//
// What stays is listed below with why. Fixing a file shrinks its count here; the list only shrinks.
import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

const TOKEN = /(?<![\w-])((?:class:)|[a-z-]*:)?(grid-cols-(\d+)|grid-cols-\[([^\]\s]+)\]|(?:min-)?w-\[(\d+)px\]|max-(?:sm|md|lg|xl):[^\s"'`]+)/g;

/** Whether one class token lays out wider than a phone, unprefixed. */
export function wide(match: RegExpMatchArray): boolean {
  const [, prefix, token, n, cols, w] = match;
  if (token.startsWith('max-')) return true;
  if (prefix && prefix !== 'class:') return false;
  if (n) return Number(n) >= 3;
  if (cols) {
    const tracks = cols.split('_');
    return tracks.length >= 4 || tracks.some((c) => Number(/^(\d+)px$/.exec(c)?.[1] ?? 0) >= 200);
  }
  if (w) return Number(w) >= 300;
  return false;
}

/** file (relative to src/) → how many wide layouts it may keep, and why. */
const ALLOWED: Record<string, { count: number; why: string }> = {
  'lib/components/sessions/SessionRow.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'lib/components/settings/InstrumentsSection.svelte': { count: 2, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'lib/preview/PreviewComponent.svelte': { count: 2, why: 'dev-only /preview bench chrome (not shipped)' },
  'routes/(config)/+layout.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(config)/setup/done/+page.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'lib/components/indexer/ScanFull.svelte': { count: 2, why: 'to fix: #420 screens slice (update-phone-layout §4) — moved verbatim from the setup scan route' },
  'routes/(config)/setup/welcome/+page.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(health)/logs/+page.svelte': { count: 2, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/RecentSessions.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/activity-logs/+page.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/activity-logs/LogRow.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/activity-logs/ScheduledTaskRow.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/consolidation/+page.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/impact/+page.svelte': { count: 2, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/insights/+page.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/instruments/+page.svelte': { count: 9, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/libraries/+page.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/workspaces/+page.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(observatory)/workspaces/WorkspaceCard.svelte': { count: 2, why: 'fine: three short mini-stats fit a phone card' },
  'routes/(observatory)/workspaces/WorkspaceRow.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(preferences)/settings/InferenceAssignmentsPanel.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(preferences)/settings/metrics/+page.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(workspace)/workspace/[id]/impact/+page.svelte': { count: 2, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(workspace)/workspace/[id]/instruments/+page.svelte': { count: 2, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(workspace)/workspace/[id]/memories/+page.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(workspace)/workspace/[id]/overview/+page.svelte': { count: 1, why: 'to fix: #420 screens slice (update-phone-layout §4)' },
  'routes/(workspace)/workspace/[id]/tickets/TicketsScreen.svelte': { count: 1, why: 'fine: the grid sits inside an overflow-x-auto wrapper' },
};

function svelteFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return svelteFiles(p);
    return p.endsWith('.svelte') && !p.endsWith('.harness.svelte') ? [p] : [];
  });
}

describe('phone layout: mobile-first', () => {
  const found = new Map<string, number>();
  for (const file of svelteFiles('src')) {
    const n = [...readFileSync(file, 'utf8').matchAll(TOKEN)].filter(wide).length;
    if (n > 0) found.set(file.replace(/^src\//, ''), n);
  }

  it('the patterns: what counts as wider than a phone', () => {
    const hits = (cls: string) => [...cls.matchAll(TOKEN)].filter(wide).length;
    expect(hits('grid grid-cols-3')).toBe(1);
    expect(hits('grid grid-cols-1 md:grid-cols-3')).toBe(0);
    expect(hits('grid-cols-[280px_1fr]')).toBe(1);
    expect(hits('grid-cols-[8px_120px_1fr_auto]')).toBe(1);
    expect(hits('grid-cols-[1fr_auto]')).toBe(0);
    expect(hits('w-[220px] min-w-[320px]')).toBe(1);
    expect(hits('class:grid-cols-[280px_1fr]={open}')).toBe(1);
    expect(hits('max-md:flex-col')).toBe(1);
  });

  it('no file adds a layout wider than a phone without a phone default', () => {
    const extra = [...found].filter(([f, n]) => n > (ALLOWED[f]?.count ?? 0)).map(([f, n]) => `${f} (${n})`);
    expect(extra, 'lay out for a phone first; adjust up with sm:/md:').toEqual([]);
  });

  it('the allowlist has no stale entries', () => {
    const stale = Object.entries(ALLOWED).filter(([f, a]) => (found.get(f) ?? 0) < a.count).map(([f]) => f);
    expect(stale, 'shrink or drop these entries').toEqual([]);
  });
});
