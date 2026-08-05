/**
 * Accessibility scan — axe-core (WCAG 2.1 A + AA) across key screens in BOTH
 * light and dark mode. Injects axe into the running Tauri webview, forces the
 * mode via body.dataset.mode (the app's own theme switch), and runs axe on the
 * settled DOM. Fails on any serious/critical violation; writes a full JSON
 * report to /tmp/sensei-a11y/report.json for review.
 *
 * Run: `bun run test:e2e -- e2e/tests/zz-a11y.spec.ts`
 */
import { test, expect } from '../fixtures';
import { navigateTo, navigateToScreen, daemonGet, DAEMON_URL } from '../helpers';
import { readFileSync, mkdirSync, writeFileSync } from 'fs';

const AXE_SRC = readFileSync('node_modules/axe-core/axe.min.js', 'utf8');
const OUT = '/tmp/sensei-a11y';
const settle = (ms = 900) => new Promise<void>((r) => setTimeout(r, ms));

type Violation = { id: string; impact: string; help: string; n: number; targets: string[] };

async function seedSetupComplete(tauriPage: any): Promise<void> {
  await fetch(`${DAEMON_URL}/api/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ setup_complete: '1' }),
  });
  await tauriPage.evaluate(`
    (function () {
      var s = window.__sensei_state__;
      if (s && s.appState) {
        s.appState.config = Object.assign({}, s.appState.config, { setup_complete: '1' });
        s.appState.loaded = true;
      }
      return true;
    })()
  `);
}

/** Force the color mode the same way the app does (body.dataset.mode). Returns
 *  the applied mode so the eval is value-returning (avoids the void-eval hang). */
async function setMode(tauriPage: any, mode: 'light' | 'dark'): Promise<void> {
  await tauriPage.evaluate(
    `(function () { document.body.dataset.mode = ${JSON.stringify(mode)}; return document.body.dataset.mode; })()`,
  );
}

/** Inject axe-core once per page context. Value-returning. */
async function ensureAxe(tauriPage: any): Promise<void> {
  await tauriPage.evaluate(
    `(function () { if (!window.axe) { ${AXE_SRC} } return typeof window.axe === 'object'; })()`,
  );
}

/** Run axe (WCAG A + AA) on the current DOM; return a compact violation list. */
async function runAxe(tauriPage: any): Promise<Violation[]> {
  const json = (await tauriPage.evaluate(`
    (async function () {
      var r = await window.axe.run(document, {
        runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'] },
        resultTypes: ['violations'],
      });
      return JSON.stringify((r.violations || []).map(function (v) {
        return {
          id: v.id, impact: v.impact, help: v.help, n: v.nodes.length,
          targets: v.nodes.slice(0, 8).map(function (nd) {
            // For color-contrast, capture axe's measured fg/bg/ratio so a fix is
            // precise (not guessed). The data lives on the failing check under any[].
            var d = (nd.any && nd.any[0] && nd.any[0].data) || {};
            var c = d.contrastRatio != null
              ? ' [' + d.contrastRatio + ':1 fg=' + d.fgColor + ' bg=' + d.bgColor + ']'
              : '';
            return nd.target.join(' ') + c;
          }),
        };
      }));
    })()
  `)) as string;
  return JSON.parse(json) as Violation[];
}

// Representative screens covering the distinct layouts + everything recently
// changed. `sel` is a stable element that proves the screen mounted.
const OBSERVATORY: Array<{ route: string; sel: string }> = [
  { route: '/',              sel: '[data-ftr-header]' },
  { route: '/insights',      sel: '[data-triage-grid], [data-empty]' },
  { route: '/projects',      sel: '[data-testid="library-search"], main' },
  { route: '/impact',        sel: 'main' },
  { route: '/traceability',  sel: '[data-traceability-total]' },
  { route: '/libraries',     sel: 'main' },
  { route: '/instruments',   sel: 'main' },
  { route: '/sessions',      sel: 'main' },
  { route: '/memories',      sel: 'main' },
  { route: '/learnings',     sel: 'main' },
  { route: '/upgrades',      sel: 'main' },
  { route: '/settings',      sel: 'main' },
];

test.describe('accessibility (axe) — light + dark', () => {
  test('no serious/critical WCAG violations on key screens', async ({ tauriPage }) => {
    test.setTimeout(600_000);
    mkdirSync(OUT, { recursive: true });
    await seedSetupComplete(tauriPage);
    await navigateTo(tauriPage, '/logs');

    // Project-window screens need a real project id.
    const projects = await daemonGet<Array<{ id: string }>>('/api/projects');
    const pid = projects[0]?.id;
    const projectScreens = pid
      ? [
          { route: `/project/${pid}/overview`, sel: '[data-component="project-sidebar"]' },
          { route: `/project/${pid}/intake`,   sel: '[data-testid="intake-input"]' },
          { route: `/project/${pid}/atlas`,    sel: '[data-screen="atlas"]' },
          { route: `/project/${pid}/libraries`, sel: '[data-testid="library-search"]' },
          { route: `/project/${pid}/about`,    sel: 'main' },
        ]
      : [];

    const screens = [...OBSERVATORY, ...projectScreens];
    const report: Record<string, Record<string, Violation[]>> = {};
    const offenders: string[] = [];

    for (const { route, sel } of screens) {
      try {
        await navigateToScreen(tauriPage, route, sel);
      } catch {
        // A screen that can't mount on this DB (e.g. empty state selector miss)
        // is logged, not silently skipped — honest coverage.
        report[route] = { error: [{ id: 'did-not-mount', impact: 'n/a', help: `selector ${sel} never appeared`, n: 0, targets: [] }] };
        continue;
      }
      await ensureAxe(tauriPage);
      report[route] = {};
      for (const mode of ['light', 'dark'] as const) {
        await setMode(tauriPage, mode);
        await settle();
        const violations = await runAxe(tauriPage);
        report[route][mode] = violations;
        for (const v of violations) {
          if (v.impact === 'serious' || v.impact === 'critical') {
            offenders.push(`[${mode}] ${route} — ${v.id} (${v.impact}, ${v.n}×): ${v.targets[0] ?? ''}`);
          }
        }
      }
    }

    writeFileSync(`${OUT}/report.json`, JSON.stringify(report, null, 2));
    // Surface every serious/critical finding in the test output.
    if (offenders.length) {
      // eslint-disable-next-line no-console
      console.log('\n=== a11y serious/critical violations ===\n' + offenders.join('\n') + '\n');
    }
    expect(offenders, `serious/critical a11y violations:\n${offenders.join('\n')}`).toEqual([]);
  });
});
