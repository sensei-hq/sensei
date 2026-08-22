/**
 * Project window — Metrics pane.
 *
 * Verifies the dedicated /project/{id}/metrics screen: the sidebar exposes a
 * Metrics nav item, the pane mounts (cards / empty / error state — never blank),
 * and for a project with computed metrics every card shows a real, non-empty
 * value. Runs against the (isolated) e2e daemon like the other project-window
 * flows; skips when no project / no metrics exist.
 */
import { test, expect } from '../fixtures';
import { navigateToScreen, DAEMON_URL } from '../helpers';

type Project = { id: string; name: string };

async function safeJson<T>(url: string, fallback: T): Promise<T> {
  try {
    const res = await fetch(url);
    if (!res.ok) return fallback;
    const text = await res.text();
    return text ? (JSON.parse(text) as T) : fallback;
  } catch {
    return fallback;
  }
}

async function pickTestProject(): Promise<Project | null> {
  const body = await safeJson<Project[] | { projects: Project[] }>(`${DAEMON_URL}/api/projects`, []);
  const list: Project[] = Array.isArray(body) ? body : (body.projects ?? []);
  for (const name of ['sensei', 'rokkit']) {
    const p = list.find((x) => x.name === name);
    if (p) return p;
  }
  return list[0] ?? null;
}

test.describe('Project window — Metrics', () => {
  test('sidebar exposes a Metrics nav item and the pane mounts', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) {
      test.skip(true, 'no projects registered');
      return;
    }

    await navigateToScreen(
      tauriPage,
      `/project/${project.id}/overview`,
      '[data-component="project-sidebar"]',
    );

    const hasMetrics = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-component="project-sidebar"] .proj-nav-item'))
        .some(a => a.textContent && a.textContent.includes('Metrics'))`,
    );
    expect(hasMetrics).toBe(true);

    // The pane must mount in one of its three states — never a blank screen.
    await navigateToScreen(
      tauriPage,
      `/project/${project.id}/metrics`,
      '[data-component="project-shell"]',
    );
    const state = await tauriPage.evaluate(
      `document.querySelector('[data-component="metric-card"]') ? 'cards'
        : document.querySelector('[data-component="metrics-empty"]') ? 'empty'
        : document.querySelector('[data-component="metrics-error"]') ? 'error'
        : 'none'`,
    );
    expect(state).not.toBe('none');
  });

  test('metrics cards render real, non-empty values for a project with data', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) {
      test.skip(true, 'no projects registered');
      return;
    }

    const data = await safeJson<{ metrics: unknown[] }>(
      `${DAEMON_URL}/api/projects/${project.id}/metrics`,
      { metrics: [] },
    );
    if (!data.metrics.length) {
      test.skip(true, 'no metrics computed for the test project');
      return;
    }

    await navigateToScreen(
      tauriPage,
      `/project/${project.id}/metrics`,
      '[data-component="metric-card"]',
    );

    const values = (await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-component="metric-value"]'))
        .map(el => el.textContent && el.textContent.trim())`,
    )) as string[];
    expect(values.length).toBeGreaterThan(0);
    expect(values.every((v) => typeof v === 'string' && v.length > 0)).toBe(true);
  });

  test('a signal detail names where you are: metrics · family · signal', async ({ tauriPage }) => {
    const project = await pickTestProject();
    if (!project) {
      test.skip(true, 'no projects registered');
      return;
    }

    const data = await safeJson<{ metrics: Array<{ metric?: string }> }>(
      `${DAEMON_URL}/api/projects/${project.id}/metrics`,
      { metrics: [] },
    );
    const key = data.metrics.find((m) => m.metric)?.metric;
    if (!key) {
      test.skip(true, 'no metrics computed for the test project');
      return;
    }

    await navigateToScreen(
      tauriPage,
      `/project/${project.id}/metrics/${key}`,
      '[data-component="signal-detail"]',
    );

    // Three segments, and the last one is a real signal name — not an empty
    // trail rendered because the lookup missed.
    const crumb = (await tauriPage.evaluate(
      `(document.querySelector('[data-component="signal-breadcrumb"]')?.textContent ?? '').trim()`,
    )) as string;
    expect(crumb.startsWith('metrics ·')).toBe(true);
    expect(crumb.split('·').map((s) => s.trim()).filter(Boolean).length).toBe(3);
  });
});
