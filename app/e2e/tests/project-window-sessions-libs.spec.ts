/**
 * Project window — Sessions + Libraries screens (T3 Slices 1.4 + 1.5).
 *
 * The two easy-screen enhancements landed on top of the existing shell:
 *   • Sessions gains a real table with date / model / turns / corrections
 *     / FTR / outcome columns plus a client-side FTR filter.
 *   • Libraries surfaces the T1a version-conflict signal as a banner,
 *     adds wrapped / local badges per row, and gains a name/ecosystem
 *     search + a filter chip set.
 *
 * The daemon exposes `provider` + `model` per session, `hasDocs` +
 * `pageCount` + `localSource` per library, and the version-conflicts
 * view is queried directly.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

async function safeJson<T>(url: string, fallback: T): Promise<T> {
  try {
    const res = await fetch(url);
    if (!res.ok) return fallback;
    const text = await res.text();
    if (!text) return fallback;
    return JSON.parse(text) as T;
  } catch { return fallback; }
}

type Project = { id: string; name: string };
async function pickProject(): Promise<Project | null> {
  const projects = await safeJson<Project[]>(`${DAEMON_URL}/api/projects`, []);
  return projects.find(p => p.name === 'sensei') ?? projects[0] ?? null;
}

test.describe('Project window — Sessions digest (project-scoped)', () => {
  // The project Sessions screen now shares the Observatory Sessions digest
  // primitives (charts + rows), scoped to one project via
  // /api/sessions?project=<id>&range=<range>. Markers mirror the observatory
  // screen: [data-session-row], [data-testid="chart-chips"], range chips.
  test('renders only this project\'s sessions — no cross-project leak', async ({ tauriPage }) => {
    const project = await pickProject();
    if (!project) { test.skip(true, 'no project'); return; }
    // Widest window so the rendered 90d slice matches what we fetch. The endpoint
    // may return a bare array OR a { sessions: [...] } envelope — normalise both
    // (a raw array assumption threw `scoped.map is not a function`).
    const scopedRaw = await safeJson<unknown>(
      `${DAEMON_URL}/api/sessions?project=${project.id}&range=90d`, [],
    );
    const scoped: Array<{ id: string }> = Array.isArray(scopedRaw)
      ? (scopedRaw as Array<{ id: string }>)
      : ((scopedRaw as { sessions?: Array<{ id: string }> })?.sessions ?? []);
    if (scoped.length === 0) { test.skip(true, 'no sessions on project'); return; }

    await navigateTo(tauriPage, `/project/${project.id}/sessions`);
    await tauriPage.waitForSelector('[data-testid="project-sessions"]', 15_000);
    // Widen the range to 90d to match the scoped fetch above, then read rows.
    await tauriPage.locator('[data-testid="range-90d"]').click();
    await tauriPage.waitForSelector('[data-session-row]', 15_000);

    const rowIds = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-session-row]'))
        .map(el => el.getAttribute('data-session-row'))
    `) as string[];
    expect(rowIds.length).toBeGreaterThan(0);
    // Every rendered row must belong to this project's scoped set.
    const scopedIds = new Set(scoped.map(s => s.id));
    for (const id of rowIds) expect(scopedIds.has(id)).toBe(true);
  });

  test('all four chart variants render through the chip group', async ({ tauriPage }) => {
    const project = await pickProject();
    if (!project) { test.skip(true, 'no project'); return; }

    await navigateTo(tauriPage, `/project/${project.id}/sessions`);
    await tauriPage.waitForSelector('[data-testid="chart-chips"]', 15_000);

    // trend is the default; switching to each of the other three must swap the
    // chart body without error (charts render even with an empty slice).
    for (const [chip, component] of [
      ['trend', 'trend-chart'],
      ['stream', 'stream-chart'],
      ['constellation', 'constellation-chart'],
      ['bands', 'bands-chart'],
    ] as const) {
      await tauriPage.locator(`[data-testid="chip-${chip}"]`).click();
      await tauriPage.waitForSelector(`[data-testid="chart-body"] [data-component="${component}"]`, 10_000);
    }
  });
});

test.describe('Project window — Libraries with version conflicts + badges (T3 Slice 1.5)', () => {
  test('conflicts banner surfaces the T1a version-conflict signal', async ({ tauriPage }) => {
    // Pick a project that HAS version conflicts. rokkit typically has many
    // (24 on the local daemon at last check).
    const projects = await safeJson<Project[]>(`${DAEMON_URL}/api/projects`, []);
    let target: { project: Project; conflicts: Array<{ library_name: string }> } | null = null;
    for (const name of ['rokkit', 'sensei']) {
      const p = projects.find(x => x.name === name);
      if (!p) continue;
      const c = await safeJson<{ conflicts: Array<{ library_name: string }> }>(
        `${DAEMON_URL}/api/projects/${p.id}/library-version-conflicts`, { conflicts: [] },
      );
      if (c.conflicts.length > 0) { target = { project: p, conflicts: c.conflicts }; break; }
    }
    if (!target) { test.skip(true, 'no project has version conflicts'); return; }

    await navigateTo(tauriPage, `/project/${target.project.id}/libraries`);
    // Banner + count badge both render when conflicts exist.
    await tauriPage.waitForSelector('[data-testid="library-conflicts-banner"]', 15_000);
    await expect(tauriPage.locator('[data-testid="library-conflicts-count"]')).toBeVisible({ timeout: 5_000 });

    // At least one conflict row is listed by name.
    const firstName = target.conflicts[0].library_name;
    await expect(tauriPage.locator(`[data-testid="library-conflict-${firstName}"]`)).toBeVisible({ timeout: 5_000 });
  });

  test('filter chip narrows to Wrapped libraries only', async ({ tauriPage }) => {
    const project = await pickProject();
    if (!project) { test.skip(true, 'no project'); return; }
    const resp = await safeJson<{ libraries: Array<{ name: string; hasDocs?: boolean }> }>(
      `${DAEMON_URL}/api/projects/${project.id}/libraries`, { libraries: [] },
    );
    const wrapped = resp.libraries.filter(l => l.hasDocs);
    if (wrapped.length === 0) { test.skip(true, 'no wrapped libraries on project'); return; }

    await navigateTo(tauriPage, `/project/${project.id}/libraries`);
    await tauriPage.waitForSelector('[data-testid="library-list"]', 15_000);

    await tauriPage.locator('[data-testid="library-filter-wrapped"]').click();
    await new Promise<void>(r => setTimeout(r, 250));

    // Every visible row must carry the wrapped badge.
    const visibleNames = await tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-testid^="library-row-"]'))
        .map(el => el.dataset.testid.replace('library-row-', ''))
    `) as string[];
    expect(visibleNames.length).toBeGreaterThan(0);
    const wrappedNames = new Set(wrapped.map(l => l.name));
    for (const n of visibleNames) expect(wrappedNames.has(n)).toBe(true);
  });
});
