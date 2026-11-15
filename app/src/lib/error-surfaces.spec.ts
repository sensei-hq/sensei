// Every error, refusal and wait renders through the shared Notice
// (docs/architecture/notice.md, docs/plan/notice-adoption.md P5). This spec keeps
// it that way: a NEW red line, red box or `role="alert"` written by hand fails
// here, with a pointer to Notice.
//
// What stays is listed below, each with why. They are not error messages: a
// tone map, a count, a verdict, a status dot. Adding to this list needs the
// same justification — a hand-written error message never belongs on it.
import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

const DANGER = /text-danger|bg-danger|border-danger|role="alert"/g;

/** file (relative to src/) → how many danger-coloured uses it may keep, and why. */
const ALLOWED: Record<string, { count: number; why: string }> = {
  'lib/components/MetricCard.svelte': { count: 1, why: "the 'bad' value tone of a metric" },
  'lib/components/SignalCard.svelte': { count: 2, why: "the 'warn' signal tone map (fill and edge)" },
  'routes/(workspace)/workspace/[id]/metrics/HealthRadar.svelte': { count: 1, why: 'a danger-toned axis label' },
  'routes/(workspace)/workspace/[id]/metrics/ActionItems.svelte': { count: 1, why: 'the high-priority chip tone' },
  'routes/(workspace)/workspace/[id]/traceability/+page.svelte': { count: 1, why: 'the "N broken" count' },
  'routes/(observatory)/traceability/+page.svelte': { count: 1, why: 'the "N broken" count' },
  'routes/(workspace)/workspace/[id]/integrations/IntegrationsScreen.svelte': { count: 1, why: "a tracker's error status dot" },
  'routes/(workspace)/workspace/[id]/impact/+page.svelte': { count: 2, why: 'the Failure verdict button and verdict colour (rokkit Button has no success/warning variant)' },
  'routes/(observatory)/+page.svelte': { count: 1, why: 'FTR-unavailable stat placeholder (the value itself; the reason is its title)' },
  'routes/(observatory)/workspaces/WorkspaceCard.svelte': { count: 1, why: 'FTR-unavailable stat placeholder' },
  'routes/(observatory)/workspaces/WorkspaceRow.svelte': { count: 1, why: 'FTR-unavailable stat placeholder' },
  'routes/(observatory)/instruments/+page.svelte': { count: 1, why: "a 'warn' insight's tone" },
  'routes/(observatory)/learnings/MemoryDetail.svelte': { count: 2, why: 'the challenged/dismissed status chip tones' },
  'routes/(preferences)/settings/dojo/DojoRepos.svelte': { count: 1, why: "a failing repository's status dot" },
  'routes/(workspace)/workspace/[id]/metrics/MetricSettingsPanel.svelte': { count: 5, why: 'new on develop in L1\'s metrics path; L1 moves it onto Notice (reported to L0), not migrated from L4' },
};

function svelteFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return svelteFiles(p);
    return p.endsWith('.svelte') && !p.endsWith('.harness.svelte') ? [p] : [];
  });
}

describe('error surfaces render through Notice', () => {
  const found = new Map<string, number>();
  for (const file of svelteFiles('src')) {
    const n = readFileSync(file, 'utf8').match(DANGER)?.length ?? 0;
    if (n > 0) found.set(file.replace(/^src\//, ''), n);
  }

  it('no file colours an error by hand — say it with <Notice> (@sensei/ui)', () => {
    const extra = [...found].filter(([f, n]) => n > (ALLOWED[f]?.count ?? 0)).map(([f, n]) => `${f} (${n})`);
    expect(extra, 'hand-written error colour or role="alert"; use Notice instead').toEqual([]);
  });

  it('the allowlist has no stale entries', () => {
    const stale = Object.entries(ALLOWED).filter(([f, a]) => (found.get(f) ?? 0) < a.count).map(([f]) => f);
    expect(stale, 'shrink or drop these entries').toEqual([]);
  });
});
