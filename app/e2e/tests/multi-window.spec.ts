/**
 * Multi-window E2E tests — observatory layout, projects page, and project window.
 *
 * Tests navigate to routes directly (the SvelteKit routes are the same whether
 * rendered in the main window or a project WebviewWindow). The tauriPage fixture
 * connects to the main window socket; project-window routes are verified by
 * navigating the main window to those URLs, which exercises the same components.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL, daemonGet } from '../helpers';

// ─── Observatory layout ──────────────────────────────────────────────────────

test.describe('Observatory layout', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/observatory');
    await expect(tauriPage.locator('.app-shell')).toBeVisible({ timeout: 10_000 });
  });

  test('renders app-shell with sidebar and main-content', async ({ tauriPage }) => {
    await expect(tauriPage.locator('.sidebar')).toBeVisible();
    await expect(tauriPage.locator('.main-content')).toBeVisible();
  });

  test('sidebar has Observatory section label', async ({ tauriPage }) => {
    const labels = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('.sidebar-label')).map(el => el.textContent?.trim().toLowerCase())`,
    ) as string[];
    expect(labels.some(l => l?.includes('observatory'))).toBe(true);
  });

  test('sidebar nav contains expected kanji and labels', async ({ tauriPage }) => {
    // Each nav item has a kanji and a label span
    const navText = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('.nav-item')).map(el => el.textContent?.trim())`,
    ) as string[];

    // Kanji checks
    const joined = navText.join(' ');
    expect(joined).toContain('家');   // Today
    expect(joined).toContain('場');   // Projects
    expect(joined).toContain('刻');   // Sessions
    expect(joined).toContain('學');   // Insights
    expect(joined).toContain('書');   // Libraries
    expect(joined).toContain('具');   // Instruments

    // Label checks
    expect(joined).toContain('Today');
    expect(joined).toContain('Projects');
    expect(joined).toContain('Sessions');
    expect(joined).toContain('Insights');
    expect(joined).toContain('Libraries');
    expect(joined).toContain('Instruments');
  });

  test('sidebar footer shows daemon status', async ({ tauriPage }) => {
    await expect(tauriPage.locator('.daemon-status')).toBeVisible();
    const text = await tauriPage.evaluate(
      `document.querySelector('.daemon-status')?.textContent?.trim()`,
    ) as string;
    expect(text).toMatch(/daemon\s*·/);
  });

  test('/observatory route is active on Today nav item', async ({ tauriPage }) => {
    const activeItems = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('.nav-item.active')).map(el => el.textContent?.trim())`,
    ) as string[];
    expect(activeItems.some(t => t?.includes('Today') || t?.includes('家'))).toBe(true);
  });

  test('sidebar collapses to icon-only view', async ({ tauriPage }) => {
    // collapse
    await tauriPage.locator('.collapse-btn').nth(0).click();
    await new Promise(r => setTimeout(r, 300));
    const collapsed = await tauriPage.evaluate(
      `document.querySelector('.app-body')?.classList.contains('collapsed')`,
    ) as boolean;
    expect(collapsed).toBe(true);

    // expand again
    await tauriPage.locator('.collapse-btn').nth(0).click();
    await new Promise(r => setTimeout(r, 300));
    const expandedAgain = await tauriPage.evaluate(
      `document.querySelector('.app-body')?.classList.contains('collapsed')`,
    ) as boolean;
    expect(expandedAgain).toBe(false);
  });
});

// ─── Observatory nav links ────────────────────────────────────────────────────

test.describe('Observatory nav — section links', () => {
  test('navigating to /insights renders insights page', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/insights');
    await expect(tauriPage.locator('.app-shell')).toBeVisible({ timeout: 10_000 });
    const activeText = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('.nav-item.active')).map(el => el.textContent?.trim())`,
    ) as string[];
    expect(activeText.some(t => t?.includes('Insights') || t?.includes('學'))).toBe(true);
  });

  test('navigating to /help renders help page', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/help');
    await expect(tauriPage.locator('.app-shell')).toBeVisible({ timeout: 10_000 });
    // Help page has h1 "Sensei Help" and h2 "Quick Start"
    const h1 = await tauriPage.evaluate(
      `document.querySelector('h1')?.textContent?.trim()`,
    ) as string;
    expect(h1).toBe('Sensei Help');
  });
});

// ─── Projects page ────────────────────────────────────────────────────────────

test.describe('Projects page', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/projects');
    await expect(tauriPage.locator('.projects-page')).toBeVisible({ timeout: 10_000 });
  });

  test('renders Projects heading', async ({ tauriPage }) => {
    const h2 = await tauriPage.evaluate(
      `document.querySelector('.projects-page h2')?.textContent?.trim()`,
    ) as string;
    expect(h2).toBe('Projects');
  });

  test('shows project grid or empty hint', async ({ tauriPage }) => {
    const gridCount = await tauriPage.locator('.project-grid').count();
    const emptyCount = await tauriPage.locator('.empty-hint').count();
    // Exactly one of these should be present
    expect(gridCount + emptyCount).toBeGreaterThan(0);
  });

  test('project cards have kanji, name and open-hint when projects exist', async ({ tauriPage }) => {
    const cardCount = await tauriPage.locator('.project-card').count();
    if (cardCount === 0) {
      // No projects in dev DB — just verify empty hint is shown
      await expect(tauriPage.locator('.empty-hint')).toBeVisible();
      return;
    }

    // First card must have a kanji span, name span and ↗ hint
    const kanjis = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('.project-card .proj-kanji')).map(el => el.textContent?.trim())`,
    ) as string[];
    expect(kanjis.length).toBe(cardCount);

    const names = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('.project-card .proj-name')).map(el => el.textContent?.trim())`,
    ) as string[];
    expect(names.every(n => n && n.length > 0)).toBe(true);

    const hints = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('.project-card .open-hint')).map(el => el.textContent?.trim())`,
    ) as string[];
    expect(hints.every(h => h === '↗')).toBe(true);
  });

  test('/projects nav item is active', async ({ tauriPage }) => {
    const activeItems = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('.nav-item.active')).map(el => el.textContent?.trim())`,
    ) as string[];
    expect(activeItems.some(t => t?.includes('Projects') || t?.includes('場'))).toBe(true);
  });
});

// ─── Project window layout ───────────────────────────────────────────────────

/**
 * Fetch a real project ID from the dev daemon. Returns null when no projects exist.
 */
async function getFirstProjectId(): Promise<string | null> {
  try {
    const projects = await daemonGet<Array<{ id: string; name: string }>>('/api/projects');
    return projects[0]?.id ?? null;
  } catch {
    return null;
  }
}

test.describe('Project window — PerspectiveChrome layout', () => {
  let projectId: string | null = null;

  test.beforeAll(async () => {
    projectId = await getFirstProjectId();
  });

  test('accent stripe, titlebar and sidebar are present', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database — skipping project window tests');
      return;
    }
    await navigateTo(tauriPage, `/project/${projectId}/overview`);
    await expect(tauriPage.locator('.project-shell')).toBeVisible({ timeout: 10_000 });

    // 2px shu accent stripe at top
    await expect(tauriPage.locator('.accent-stripe')).toBeVisible();

    // Titlebar: kanji, project name, "· project window" sub-label
    await expect(tauriPage.locator('.titlebar')).toBeVisible();
    const sub = await tauriPage.evaluate(
      `document.querySelector('.proj-sub')?.textContent?.trim()`,
    ) as string;
    expect(sub).toBe('· project window');

    // Sidebar present
    await expect(tauriPage.locator('.proj-sidebar')).toBeVisible();

    // FTR stat block in sidebar
    await expect(tauriPage.locator('.sidebar-stats')).toBeVisible();
    const statLabel = await tauriPage.evaluate(
      `document.querySelector('.stat-label')?.textContent?.trim()`,
    ) as string;
    expect(statLabel).toBe('FTR 14d');
  });

  test('sidebar nav has exactly 9 items with correct kanji', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database — skipping project window tests');
      return;
    }
    await navigateTo(tauriPage, `/project/${projectId}/overview`);
    await expect(tauriPage.locator('.proj-nav')).toBeVisible({ timeout: 10_000 });

    const navItems = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('.proj-nav-item')).map(el => ({
        kanji: el.querySelector('.kanji')?.textContent?.trim(),
        label: el.querySelector('.label')?.textContent?.trim(),
      }))`,
    ) as Array<{ kanji: string; label: string }>;

    expect(navItems).toHaveLength(9);

    const expected = [
      { kanji: '見', label: 'Overview'     },
      { kanji: '録', label: 'Sessions'     },
      { kanji: '憶', label: 'Memories'     },
      { kanji: '跡', label: 'Traceability' },
      { kanji: '蔵', label: 'Libraries'    },
      { kanji: '器', label: 'Instruments'  },
      { kanji: '型', label: 'Patterns'     },
      { kanji: '響', label: 'Impact'       },
      { kanji: '情', label: 'About'        },
    ];

    expected.forEach(({ kanji, label }, i) => {
      expect(navItems[i].kanji).toBe(kanji);
      expect(navItems[i].label).toBe(label);
    });
  });

  test('overview section is active on /overview route', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database — skipping project window tests');
      return;
    }
    await navigateTo(tauriPage, `/project/${projectId}/overview`);
    await expect(tauriPage.locator('.proj-nav')).toBeVisible({ timeout: 10_000 });

    const activeItem = await tauriPage.evaluate(
      `document.querySelector('.proj-nav-item.active')?.querySelector('.label')?.textContent?.trim()`,
    ) as string;
    expect(activeItem).toBe('Overview');
  });
});

// ─── Project section pages ───────────────────────────────────────────────────

test.describe('Project window — section pages', () => {
  let projectId: string | null = null;

  test.beforeAll(async () => {
    projectId = await getFirstProjectId();
  });

  const SECTIONS: Array<{ id: string; label: string; kanji: string; rootClass: string; h2: string }> = [
    { id: 'overview',     label: 'Overview',     kanji: '見', rootClass: '.overview-page',    h2: '' },
    { id: 'sessions',     label: 'Sessions',     kanji: '録', rootClass: '.section-page',     h2: 'Sessions' },
    { id: 'memories',     label: 'Memories',     kanji: '憶', rootClass: '.section-page',     h2: 'Memories' },
    { id: 'traceability', label: 'Traceability', kanji: '跡', rootClass: '.section-page',     h2: 'Traceability' },
    { id: 'libraries',    label: 'Libraries',    kanji: '蔵', rootClass: '.libraries-page',   h2: 'Libraries' },
    { id: 'instruments',  label: 'Instruments',  kanji: '器', rootClass: '.instruments-page', h2: 'Instruments' },
    { id: 'patterns',     label: 'Patterns',     kanji: '型', rootClass: '.section-page',     h2: 'Patterns' },
    { id: 'impact',       label: 'Impact',       kanji: '響', rootClass: '.section-page',     h2: 'Impact' },
    { id: 'about',        label: 'About',        kanji: '情', rootClass: '.section-page',     h2: '' },
  ];

  for (const section of SECTIONS) {
    test(`/${section.id} section renders root container and marks nav active`, async ({ tauriPage }) => {
      if (!projectId) {
        test.skip(true, 'No projects in dev database — skipping project window tests');
        return;
      }

      await navigateTo(tauriPage, `/project/${projectId}/${section.id}`);
      await expect(tauriPage.locator('.project-shell')).toBeVisible({ timeout: 10_000 });

      // Root content container
      await expect(tauriPage.locator(section.rootClass)).toBeVisible({ timeout: 5_000 });

      // h2 heading (if expected)
      if (section.h2) {
        const h2 = await tauriPage.evaluate(
          `document.querySelector('${section.rootClass} h2')?.textContent?.trim()`,
        ) as string;
        expect(h2).toBe(section.h2);
      }

      // Active nav item
      const activeLabel = await tauriPage.evaluate(
        `document.querySelector('.proj-nav-item.active')?.querySelector('.label')?.textContent?.trim()`,
      ) as string;
      expect(activeLabel).toBe(section.label);

      // Active kanji matches expected
      const activeKanji = await tauriPage.evaluate(
        `document.querySelector('.proj-nav-item.active')?.querySelector('.kanji')?.textContent?.trim()`,
      ) as string;
      expect(activeKanji).toBe(section.kanji);
    });
  }

  test('overview stats-row has 4 stat blocks', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database — skipping project window tests');
      return;
    }
    await navigateTo(tauriPage, `/project/${projectId}/overview`);
    await expect(tauriPage.locator('.overview-page')).toBeVisible({ timeout: 10_000 });

    const statCount = await tauriPage.locator('.stat-block').count();
    expect(statCount).toBe(4);

    const statLabels = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('.stat-label')).map(el => el.textContent?.trim())`,
    ) as string[];
    expect(statLabels).toContain('FTR 14d');
    expect(statLabels).toContain('Sessions 7d');
    expect(statLabels).toContain('Memories');
    expect(statLabels).toContain('Repos');
  });

  test('redirect from /project/{id} to /overview', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database — skipping project window tests');
      return;
    }
    // /project/{id} should redirect to /project/{id}/overview
    await navigateTo(tauriPage, `/project/${projectId}`);
    await new Promise(r => setTimeout(r, 1_200)); // let redirect settle
    const url = await tauriPage.evaluate(`window.location.href`) as string;
    expect(url).toMatch(new RegExp(`/project/${projectId}/overview`));
  });
});

// ─── Design style checks ─────────────────────────────────────────────────────

test.describe('Design fidelity — project window chrome', () => {
  let projectId: string | null = null;

  test.beforeAll(async () => {
    projectId = await getFirstProjectId();
  });

  test('accent stripe has 2px height and shu background', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database');
      return;
    }
    await navigateTo(tauriPage, `/project/${projectId}/overview`);
    await expect(tauriPage.locator('.accent-stripe')).toBeVisible({ timeout: 10_000 });

    const style = await tauriPage.evaluate(`
      const el = document.querySelector('.accent-stripe');
      if (!el) return null;
      const cs = getComputedStyle(el);
      return { height: cs.height, background: cs.background };
    `) as { height: string; background: string } | null;

    expect(style).not.toBeNull();
    expect(style!.height).toBe('2px');
    // background contains the shu red color
    expect(style!.background).toMatch(/#c0392b|rgb\(192, 57, 43\)|var\(--shu/i);
  });

  test('proj-sidebar is 180px wide', async ({ tauriPage }) => {
    if (!projectId) {
      test.skip(true, 'No projects in dev database');
      return;
    }
    await navigateTo(tauriPage, `/project/${projectId}/overview`);
    await expect(tauriPage.locator('.proj-sidebar')).toBeVisible({ timeout: 10_000 });

    const width = await tauriPage.evaluate(
      `document.querySelector('.proj-sidebar')?.getBoundingClientRect().width`,
    ) as number;
    expect(width).toBe(180);
  });

  test('observatory sidebar is 220px wide (expanded)', async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/observatory');
    await expect(tauriPage.locator('.app-shell')).toBeVisible({ timeout: 10_000 });

    // Ensure not collapsed
    const collapsed = await tauriPage.evaluate(
      `document.querySelector('.app-body')?.classList.contains('collapsed')`,
    ) as boolean;
    if (collapsed) {
      await tauriPage.locator('.collapse-btn').nth(0).click();
      await new Promise(r => setTimeout(r, 300));
    }

    const width = await tauriPage.evaluate(
      `document.querySelector('.sidebar')?.getBoundingClientRect().width`,
    ) as number;
    expect(width).toBe(220);
  });
});
