/**
 * Multi-window E2E — observatory shell, workspaces page, and workspace window.
 *
 * Selectors are the app's stable semantic hooks (data-component / data-* +
 * rokkit List's data-list-* / data-item-*), not utility classes — so they
 * survive styling churn and are shared with the inspect harness. Routes are
 * driven with navigateToScreen (retries through the health gate).
 */

import { test, expect } from '../fixtures';
import { navigateTo, navigateToScreen, DAEMON_URL, daemonGet, seedSetupComplete } from '../helpers';

// ─── Observatory shell ─────────────────────────────────────────────────────

// Every screen here sits behind the setup gate; a spec before this one may have reset it.
test.beforeEach(async ({ tauriPage }) => { await seedSetupComplete(tauriPage); });

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

  test('sidebar nav lists the Today and Workspaces destinations', async ({ tauriPage }) => {
    // Nav is a rokkit <List>: each entry is an anchor with a [data-item-label].
    // Today + Workspaces are always-visible top-level anchors (the "Review" group
    // is hidden in Focus mode, so we don't assert those here).
    const labels = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-component="observatory-sidebar"] [data-item-label]'))
        .map(el => el.textContent && el.textContent.trim())`,
    ) as string[];
    expect(labels).toContain('Today');
    expect(labels).toContain('Workspaces');
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

// ─── Workspaces page ──────────────────────────────────────────────────────────

test.describe('Workspaces page', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/workspaces', '[data-screen="workspaces"]');
  });

  test('renders the Workspaces heading', async ({ tauriPage }) => {
    const text = await tauriPage.evaluate(
      `document.querySelector('[data-screen="workspaces"]')?.textContent ?? ''`,
    ) as string;
    expect(text).toContain('Workspaces');
    expect(text).toContain('All the places you work.');
  });

  test('shows workspace cards, or an honest empty state', async ({ tauriPage }) => {
    const cards = await tauriPage
      .locator('[data-screen="workspaces"] [data-workspace-card], [data-screen="workspaces"] [data-workspace-row]')
      .count();
    if (cards === 0) {
      const text = await tauriPage.evaluate(
        `document.querySelector('[data-screen="workspaces"]')?.textContent ?? ''`,
      ) as string;
      expect(text).toMatch(/No workspaces/i);
      return;
    }
    expect(cards).toBeGreaterThan(0);

    // Each card carries an icon glyph and a non-empty name.
    const names = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-workspace-card], [data-workspace-row]'))
        .map(el => el.textContent && el.textContent.trim())`,
    ) as string[];
    expect(names.every((n) => n && n.length > 0)).toBe(true);
  });

  test('Workspaces is the active nav item', async ({ tauriPage }) => {
    const active = await tauriPage.evaluate(
      `document.querySelector('[data-component="observatory-sidebar"] [data-active] [data-item-label]')?.textContent?.trim()
        ?? document.querySelector('[data-component="observatory-sidebar"] [aria-current="page"] [data-item-label]')?.textContent?.trim()`,
    ) as string;
    expect(active).toBe('Workspaces');
  });
});

// ─── Workspace window ─────────────────────────────────────────────────────────

/** Fetch a real workspace ID from the daemon. Null when none exist. */
async function getFirstWorkspaceId(): Promise<string | null> {
  try {
    const workspaces = await daemonGet<Array<{ id: string; name: string }>>('/api/workspaces');
    return workspaces[0]?.id ?? null;
  } catch {
    return null;
  }
}

test.describe('Workspace window — chrome', () => {
  let workspaceId: string | null = null;

  test.beforeAll(async () => {
    workspaceId = await getFirstWorkspaceId();
  });

  test('titlebar and sidebar identity are present', async ({ tauriPage }) => {
    if (!workspaceId) {
      test.skip(true, 'No workspaces in dev database — skipping workspace window tests');
      return;
    }
    await navigateToScreen(tauriPage, `/workspace/${workspaceId}/overview`, '[data-component="workspace-shell"]');

    // Titlebar: workspace name + "· workspace window" sub-label
    await expect(tauriPage.locator('[data-component="workspace-titlebar"]')).toBeVisible();
    const sub = await tauriPage.evaluate(
      `document.querySelector('[data-component="workspace-titlebar"] span:last-child')?.textContent?.trim()`,
    ) as string;
    expect(sub).toBe('· workspace window');

    // Sidebar leads with the workspace identity (icon + name), not a bare FTR number.
    await expect(tauriPage.locator('[data-component="workspace-sidebar"]')).toBeVisible();
    const sidebarName = await tauriPage.evaluate(
      `document.querySelector('[data-component="sidebar-workspace-name"]')?.textContent?.trim()`,
    ) as string;
    expect(Boolean(sidebarName && sidebarName.length > 0)).toBe(true);

    // FTR is demoted into the Health readout at the foot of the sidebar.
    await expect(tauriPage.locator('[data-component="workspace-health"]')).toBeVisible();
    const healthText = await tauriPage.evaluate(
      `document.querySelector('[data-component="workspace-health"]')?.textContent?.replace(/\\s+/g, ' ').trim()`,
    ) as string;
    expect(healthText).toContain('FTR · 14d');
  });

  test('sidebar nav lists every section with correct kanji, including Metrics', async ({ tauriPage }) => {
    if (!workspaceId) {
      test.skip(true, 'No workspaces in dev database — skipping workspace window tests');
      return;
    }
    await navigateToScreen(tauriPage, `/workspace/${workspaceId}/overview`, '[data-component="workspace-sidebar"]');

    const navItems = await tauriPage.evaluate(
      `Array.from(document.querySelectorAll('[data-component="workspace-sidebar"] [data-list-item]')).map(el => ({
        kanji: el.querySelector('[data-item-icon-literal]')?.textContent?.trim(),
        label: el.querySelector('[data-item-label]')?.textContent?.trim(),
      }))`,
    ) as Array<{ kanji: string; label: string }>;

    const expected = [
      { kanji: '門', label: 'Intake'       },
      { kanji: '見', label: 'Overview'     },
      { kanji: '計', label: 'Metrics'      },
      { kanji: '録', label: 'Sessions'     },
      { kanji: '憶', label: 'Memories'     },
      { kanji: '跡', label: 'Traceability' },
      { kanji: '蔵', label: 'Libraries'    },
      { kanji: '器', label: 'Instruments'  },
      { kanji: '型', label: 'Patterns'     },
      { kanji: '響', label: 'Impact'       },
      { kanji: '札', label: 'Tickets'      },
      { kanji: '繋', label: 'Integrations' },
      { kanji: '情', label: 'About'        },
      // The Diagrams subsection (M16): one entry per diagram.
      { kanji: '構', label: 'Structure'         },
      { kanji: '輪', label: 'Wheel'             },
      { kanji: '域', label: 'Zones'             },
      { kanji: '格', label: 'Dependency matrix' },
      { kanji: '環', label: 'Cycles'            },
      { kanji: '縁', label: 'Hidden coupling'   },
      { kanji: '繁', label: 'Complexity'        },
      { kanji: '表', label: 'Database · ER'     },
    ];

    expect(navItems).toHaveLength(expected.length);
    expected.forEach(({ kanji, label }, i) => {
      expect(navItems[i].kanji).toBe(kanji);
      expect(navItems[i].label).toBe(label);
    });
  });

  test('workspace window renders with the workspace name clear of the traffic lights', async ({ tauriPage }) => {
    if (!workspaceId) {
      test.skip(true, 'No workspaces in dev database — skipping workspace window tests');
      return;
    }
    test.setTimeout(150_000);
    await navigateToScreen(tauriPage, `/workspace/${workspaceId}/overview`, '[data-component="workspace-shell"]');

    const name = await tauriPage.evaluate(
      `document.querySelector('[data-component="workspace-name"]')?.textContent?.trim()`,
    ) as string;
    expect(Boolean(name && name.length > 0 && name !== '…')).toBe(true);

    // Inset past the macOS overlay traffic lights (pl-[80px]) so it isn't hidden.
    const nameLeft = await tauriPage.evaluate(
      `document.querySelector('[data-component="workspace-name"]')?.getBoundingClientRect().left`,
    ) as number;
    expect(nameLeft).toBeGreaterThanOrEqual(78);
  });
});

// ─── Workspace section pages ───────────────────────────────────────────────────

test.describe('Workspace window — section pages', () => {
  let workspaceId: string | null = null;

  test.beforeAll(async () => {
    workspaceId = await getFirstWorkspaceId();
  });

  const SECTIONS: Array<{ id: string; label: string; kanji: string }> = [
    { id: 'overview',     label: 'Overview',     kanji: '見' },
    { id: 'sessions',     label: 'Sessions',     kanji: '録' },
    { id: 'memories',     label: 'Memories',     kanji: '憶' },
    { id: 'traceability', label: 'Traceability', kanji: '跡' },
    { id: 'libraries',    label: 'Libraries',    kanji: '蔵' },
    { id: 'instruments',  label: 'Instruments',  kanji: '器' },
    { id: 'patterns',     label: 'Patterns',     kanji: '型' },
    { id: 'impact',       label: 'Impact',       kanji: '響' },
    { id: 'about',        label: 'About',        kanji: '情' },
  ];

  for (const section of SECTIONS) {
    test(`/${section.id} mounts the section and marks its nav active`, async ({ tauriPage }) => {
      if (!workspaceId) {
        test.skip(true, 'No workspaces in dev database — skipping workspace window tests');
        return;
      }

      // <main> carries data-section={id}; wait for the specific section to mount.
      await navigateToScreen(
        tauriPage,
        `/workspace/${workspaceId}/${section.id}`,
        `[data-component="workspace-main"][data-section="${section.id}"]`,
      );

      // The rail's current entry: the List item marked active.
      const active = await tauriPage.evaluate(
        `(function(){
          const el = document.querySelector('[data-component="workspace-sidebar"] [data-list-item][data-active="true"]');
          return {
            label: el?.querySelector('[data-item-label]')?.textContent?.trim(),
            kanji: el?.querySelector('[data-item-icon-literal]')?.textContent?.trim(),
          };
        })()`,
      ) as { label: string; kanji: string };
      expect(active.label).toBe(section.label);
      expect(active.kanji).toBe(section.kanji);
    });
  }

  test('overview surfaces three stat cards (sessions · memories · doc drift)', async ({ tauriPage }) => {
    if (!workspaceId) {
      test.skip(true, 'No workspaces in dev database — skipping workspace window tests');
      return;
    }
    await navigateToScreen(
      tauriPage,
      `/workspace/${workspaceId}/overview`,
      '[data-component="workspace-main"][data-section="overview"]',
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

  test('redirect from /workspace/{id} to /overview', async ({ tauriPage }) => {
    if (!workspaceId) {
      test.skip(true, 'No workspaces in dev database — skipping workspace window tests');
      return;
    }
    await navigateTo(tauriPage, `/workspace/${workspaceId}`);
    await new Promise((r) => setTimeout(r, 1_200));
    const url = await tauriPage.evaluate(`window.location.href`) as string;
    expect(url).toMatch(new RegExp(`/workspace/${workspaceId}/overview`));
  });
});

// ─── Design fidelity ─────────────────────────────────────────────────────────

test.describe('Design fidelity — chrome widths', () => {
  let workspaceId: string | null = null;

  test.beforeAll(async () => {
    workspaceId = await getFirstWorkspaceId();
  });

  test('workspace sidebar is 220px wide, as every rail', async ({ tauriPage }) => {
    if (!workspaceId) {
      test.skip(true, 'No workspaces in dev database');
      return;
    }
    await navigateToScreen(tauriPage, `/workspace/${workspaceId}/overview`, '[data-component="workspace-sidebar"]');
    const width = await tauriPage.evaluate(
      `document.querySelector('[data-component="workspace-sidebar"]')?.getBoundingClientRect().width`,
    ) as number;
    expect(width).toBe(220);
  });

  test('observatory sidebar is 220px wide', async ({ tauriPage }) => {
    await navigateToScreen(tauriPage, '/', '[data-component="observatory-sidebar"]');
    const width = await tauriPage.evaluate(
      `document.querySelector('[data-component="observatory-sidebar"]')?.getBoundingClientRect().width`,
    ) as number;
    expect(width).toBe(220);
  });
});
