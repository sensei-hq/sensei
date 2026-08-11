/**
 * Multi-window E2E — observatory shell, projects page, and project window.
 *
 * Selectors are the app's stable semantic hooks (data-component / data-* +
 * rokkit List's data-list-* / data-item-*), not utility classes — so they
 * survive styling churn and are shared with the inspect harness. Routes are
 * driven with navigateToScreen (retries through the health gate).
 */

import { test, expect } from '../fixtures';
import { navigateTo, navigateToScreen, DAEMON_URL, daemonGet } from '../helpers';

// ─── Observatory shell ─────────────────────────────────────────────────────

test.describe('Observatory shell', () => {
  // `/` is health+setup-gated; a cold e2e boot takes ~50s to settle and
  // navigateToScreen retries for 120s. The default 60s per-test timeout is too
  // tight for the gated route, so lift it (mirrors the 214 precedent below).
  test.describe.configure({ timeout: 150_000 });

  test.beforeEach(async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/', '[data-component="observatory-sidebar"]');
  });

  test('renders the observatory sidebar and main content', async ({ tauriPage }) => {
    await expect(tauriPage.locator('[data-component="observatory-sidebar"]')).toBeVisible();
    await expect(tauriPage.locator('[data-component="observatory-main"]')).toBeVisible();
  });

  test('sidebar carries the Observatory eyebrow and daemon status', async ({ tauriPage }) => {
    const eyebrows = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-component="observatory-sidebar"] [data-component="eyebrow"]'))
        .map(el => el.textContent && el.textContent.trim().toLowerCase())`,
    ) as string[];
    expect(eyebrows.some((t) => t && t.includes('observatory'))).toBe(true);

    const sidebarText = await tauriPage.evaluate(
      `document.querySelector('[data-component="observatory-sidebar"]')?.textContent?.toLowerCase() ?? ''`,
    ) as string;
    expect(sidebarText).toContain('daemon');
  });

  test('sidebar nav lists the Today and Projects destinations', async ({ tauriPage }) => {
    // Nav is a rokkit <List>: each entry is an anchor with a [data-item-label].
    // Today + Projects are always-visible top-level anchors (the "Review" group
    // is hidden in Focus mode, so we don't assert those here).
    const labels = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-component="observatory-sidebar"] [data-item-label]'))
        .map(el => el.textContent && el.textContent.trim())`,
    ) as string[];
    expect(labels).toContain('Today');
    expect(labels).toContain('Projects');
  });

  test('Today is the active nav item on /', async ({ tauriPage }) => {
    const active = await tauriPage.evaluate(
      `document.querySelector('[data-component="observatory-sidebar"] [data-active] [data-item-label]')?.textContent?.trim()
        ?? document.querySelector('[data-component="observatory-sidebar"] [aria-current="page"] [data-item-label]')?.textContent?.trim()`,
    ) as string;
    expect(active).toBe('Today');
  });
});

// ─── Observatory nav links ──────────────────────────────────────────────────

test.describe('Observatory nav — section links', () => {
  test('navigating to /insights marks Insights active', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/insights', '[data-component="observatory-sidebar"]');
    const active = await tauriPage.evaluate(
      `document.querySelector('[data-component="observatory-sidebar"] [data-active] [data-item-label]')?.textContent?.trim()
        ?? document.querySelector('[data-component="observatory-sidebar"] [aria-current="page"] [data-item-label]')?.textContent?.trim()`,
    ) as string;
    expect(active).toBe('Insights');
  });

  test('navigating to /help renders the help page heading', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/help', '[data-component="observatory-main"]');
    const h1 = await tauriPage.evaluate(
      `document.querySelector('[data-component="observatory-main"] h1')?.textContent?.trim() ?? ''`,
    ) as string;
    expect(h1.length).toBeGreaterThan(0);
  });
});

// ─── Projects page ──────────────────────────────────────────────────────────

test.describe('Projects page', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/projects', '[data-component="projects-page"]');
  });

  test('renders the Projects heading', async ({ tauriPage }) => {
    const text = await tauriPage.evaluate(
      `document.querySelector('[data-component="projects-page"]')?.textContent ?? ''`,
    ) as string;
    expect(text).toContain('Projects');
    expect(text).toContain('All the places you work.');
  });

  test('shows project cards, or an honest empty state', async ({ tauriPage }) => {
    const cards = await tauriPage
      .locator('[data-component="projects-page"] [data-project-card], [data-component="projects-page"] [data-project-row]')
      .count();
    if (cards === 0) {
      const text = await tauriPage.evaluate(
        `document.querySelector('[data-component="projects-page"]')?.textContent ?? ''`,
      ) as string;
      expect(text).toMatch(/No projects/i);
      return;
    }
    expect(cards).toBeGreaterThan(0);

    // Each card carries an icon glyph and a non-empty name.
    const names = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-project-card], [data-project-row]'))
        .map(el => el.textContent && el.textContent.trim())`,
    ) as string[];
    expect(names.every((n) => n && n.length > 0)).toBe(true);
  });

  test('Projects is the active nav item', async ({ tauriPage }) => {
    const active = await tauriPage.evaluate(
      `document.querySelector('[data-component="observatory-sidebar"] [data-active] [data-item-label]')?.textContent?.trim()
        ?? document.querySelector('[data-component="observatory-sidebar"] [aria-current="page"] [data-item-label]')?.textContent?.trim()`,
    ) as string;
    expect(active).toBe('Projects');
  });
});

// ─── Project window ─────────────────────────────────────────────────────────

/** Fetch a real project ID from the daemon. Null when none exist. */
async function getFirstProjectId(): Promise<string | null> {
  try {
    const projects = await daemonGet<Array<{ id: string; name: string }>>('/api/projects');
    return projects[0]?.id ?? null;
  } catch {
    return null;
  }
}

test.describe('Project window — chrome', () => {
  let projectId: string | null = null;

  test.beforeAll(async () => {
    projectId = await getFirstProjectId();
  });

  test('titlebar and sidebar identity are present', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database — skipping project window tests');
      return;
    }
    await navigateToScreen(tauriPage, `/project/${projectId}/overview`, '[data-component="project-shell"]');

    // Titlebar: project name + "· project window" sub-label
    await expect(tauriPage.locator('[data-component="project-titlebar"]')).toBeVisible();
    const sub = await tauriPage.evaluate(
      `document.querySelector('[data-component="project-titlebar"] span:last-child')?.textContent?.trim()`,
    ) as string;
    expect(sub).toBe('· project window');

    // Sidebar leads with the project identity (icon + name), not a bare FTR number.
    await expect(tauriPage.locator('[data-component="project-sidebar"]')).toBeVisible();
    const sidebarName = await tauriPage.evaluate(
      `document.querySelector('[data-component="sidebar-project-name"]')?.textContent?.trim()`,
    ) as string;
    expect(Boolean(sidebarName && sidebarName.length > 0)).toBe(true);

    // FTR is demoted into the Health readout at the foot of the sidebar.
    await expect(tauriPage.locator('[data-component="project-health"]')).toBeVisible();
    const healthText = await tauriPage.evaluate(
      `document.querySelector('[data-component="project-health"]')?.textContent?.replace(/\\s+/g, ' ').trim()`,
    ) as string;
    expect(healthText).toContain('FTR · 14d');
  });

  test('sidebar nav lists every section with correct kanji, including Metrics', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database — skipping project window tests');
      return;
    }
    await navigateToScreen(tauriPage, `/project/${projectId}/overview`, '[data-component="project-sidebar"]');

    const navItems = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-component="project-sidebar"] .proj-nav-item')).map(el => ({
        kanji: el.querySelector('.kanji')?.textContent?.trim(),
        label: el.querySelector('span:not(.kanji)')?.textContent?.trim(),
      }))`,
    ) as Array<{ kanji: string; label: string }>;

    const expected = [
      { kanji: '門', label: 'Intake'       },
      { kanji: '見', label: 'Overview'     },
      { kanji: '計', label: 'Metrics'      },
      { kanji: '録', label: 'Sessions'     },
      { kanji: '憶', label: 'Memories'     },
      { kanji: '跡', label: 'Traceability' },
      { kanji: '図', label: 'Atlas'        },
      { kanji: '蔵', label: 'Libraries'    },
      { kanji: '器', label: 'Instruments'  },
      { kanji: '型', label: 'Patterns'     },
      { kanji: '響', label: 'Impact'       },
      { kanji: '情', label: 'About'        },
    ];

    expect(navItems).toHaveLength(expected.length);
    expected.forEach(({ kanji, label }, i) => {
      expect(navItems[i].kanji).toBe(kanji);
      expect(navItems[i].label).toBe(label);
    });
  });

  test('project window renders with the project name clear of the traffic lights', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database — skipping project window tests');
      return;
    }
    test.setTimeout(150_000);
    await navigateToScreen(tauriPage, `/project/${projectId}/overview`, '[data-component="project-shell"]');

    const name = await tauriPage.evaluate(
      `document.querySelector('[data-component="project-name"]')?.textContent?.trim()`,
    ) as string;
    expect(Boolean(name && name.length > 0 && name !== '…')).toBe(true);

    // Inset past the macOS overlay traffic lights (pl-[80px]) so it isn't hidden.
    const nameLeft = await tauriPage.evaluate(
      `document.querySelector('[data-component="project-name"]')?.getBoundingClientRect().left`,
    ) as number;
    expect(nameLeft).toBeGreaterThanOrEqual(78);
  });
});

// ─── Project section pages ───────────────────────────────────────────────────

test.describe('Project window — section pages', () => {
  let projectId: string | null = null;

  test.beforeAll(async () => {
    projectId = await getFirstProjectId();
  });

  const SECTIONS: Array<{ id: string; label: string; kanji: string }> = [
    { id: 'overview',     label: 'Overview',     kanji: '見' },
    { id: 'sessions',     label: 'Sessions',     kanji: '録' },
    { id: 'memories',     label: 'Memories',     kanji: '憶' },
    { id: 'traceability', label: 'Traceability', kanji: '跡' },
    { id: 'atlas',        label: 'Atlas',        kanji: '図' },
    { id: 'libraries',    label: 'Libraries',    kanji: '蔵' },
    { id: 'instruments',  label: 'Instruments',  kanji: '器' },
    { id: 'patterns',     label: 'Patterns',     kanji: '型' },
    { id: 'impact',       label: 'Impact',       kanji: '響' },
    { id: 'about',        label: 'About',        kanji: '情' },
  ];

  for (const section of SECTIONS) {
    test(`/${section.id} mounts the section and marks its nav active`, async ({ tauriPage }) => {
      if (!projectId) {
        test.skip(true, 'No projects in dev database — skipping project window tests');
        return;
      }

      // <main> carries data-section={id}; wait for the specific section to mount.
      await navigateToScreen(
        tauriPage,
        `/project/${projectId}/${section.id}`,
        `[data-component="project-main"][data-section="${section.id}"]`,
      );

      // Active nav item: .proj-nav-item.active, label is the non-.kanji span.
      const active = await tauriPage.evaluate(
        `(function(){
          const el = document.querySelector('.proj-nav-item.active');
          return {
            label: el?.querySelector('span:not(.kanji)')?.textContent?.trim(),
            kanji: el?.querySelector('.kanji')?.textContent?.trim(),
          };
        })()`,
      ) as { label: string; kanji: string };
      expect(active.label).toBe(section.label);
      expect(active.kanji).toBe(section.kanji);
    });
  }

  test('overview surfaces three stat cards (sessions · memories · doc drift)', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database — skipping project window tests');
      return;
    }
    await navigateToScreen(
      tauriPage,
      `/project/${projectId}/overview`,
      '[data-component="project-main"][data-section="overview"]',
    );

    const labels = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-component="stat-card"]'))
        .map(el => el.querySelector('div')?.textContent?.trim())`,
    ) as string[];
    expect(labels).toHaveLength(3);
    expect(labels).toContain('Sessions · 7d');
    expect(labels).toContain('Memories');
    expect(labels).toContain('Doc drift');
  });

  test('redirect from /project/{id} to /overview', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database — skipping project window tests');
      return;
    }
    await navigateTo(tauriPage, `/project/${projectId}`);
    await new Promise((r) => setTimeout(r, 1_200));
    const url = await tauriPage.evaluate(`window.location.href`) as string;
    expect(url).toMatch(new RegExp(`/project/${projectId}/overview`));
  });
});

// ─── Design fidelity ─────────────────────────────────────────────────────────

test.describe('Design fidelity — chrome widths', () => {
  let projectId: string | null = null;

  test.beforeAll(async () => {
    projectId = await getFirstProjectId();
  });

  test('project sidebar is 180px wide', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database');
      return;
    }
    await navigateToScreen(tauriPage, `/project/${projectId}/overview`, '[data-component="project-sidebar"]');
    const width = await tauriPage.evaluate(
      `document.querySelector('[data-component="project-sidebar"]')?.getBoundingClientRect().width`,
    ) as number;
    expect(width).toBe(180);
  });

  test('observatory sidebar is 220px wide', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/', '[data-component="observatory-sidebar"]');
    const width = await tauriPage.evaluate(
      `document.querySelector('[data-component="observatory-sidebar"]')?.getBoundingClientRect().width`,
    ) as number;
    expect(width).toBe(220);
  });
});
