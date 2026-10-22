/**
 * Inspect the REAL workspace Metrics screens for live workspaces (sensei, dbd,
 * rokkit). Not a regression gate — captures /tmp/sensei-shots/*.png and logs the
 * rendered signals so the "merge" landing + the master-detail drill-down can be
 * eyeballed against live data. Run: `bun run test:inspect`.
 */
import { test } from '../fixtures';
import { navigateToScreen, DAEMON_URL, shot } from '../helpers';
import { mkdirSync } from 'fs';

const DIR = '/tmp/sensei-shots';
const WORKSPACES = ['sensei', 'dbd', 'rokkit'];
const settle = (ms = 2500) => new Promise<void>((r) => setTimeout(r, ms));

type Workspace = { id: string; name: string };

async function idForName(name: string): Promise<string | null> {
  const res = await fetch(`${DAEMON_URL}/api/workspaces`);
  const body = await res.json();
  const list: Workspace[] = Array.isArray(body) ? body : (body.workspaces ?? []);
  return list.find((p) => p.name === name)?.id ?? null;
}

test.describe('inspect real workspace metrics', () => {
  for (const name of WORKSPACES) {
    test(`metrics — ${name}`, async ({ tauriPage }) => {
      mkdirSync(DIR, { recursive: true });

      const id = await idForName(name);
      if (!id) {
        test.skip(true, `no workspace named "${name}" on the real daemon`);
        return;
      }

      // ── Landing: interpreted headline + movers + uniform grid ──
      await navigateToScreen(
        tauriPage,
        `/workspace/${id}/metrics`,
        '[data-component="workspace-shell"]',
      );
      await settle();

      const landing = (await tauriPage.evaluate(`
        (() => {
          const txt = (sel) => document.querySelector(sel)?.textContent?.replace(/\\s+/g,' ').trim() ?? null;
          const cells = Array.from(document.querySelectorAll('[data-component="signal-cell"]')).map(c => ({
            key: c.getAttribute('data-signal'),
            moved: c.getAttribute('data-moved'),
            value: c.querySelector('[data-component="signal-value"]')?.textContent?.trim(),
            delta: c.querySelector('[data-component="signal-delta"]')?.textContent?.trim() || null,
          }));
          const movers = Array.from(document.querySelectorAll('[data-component="mover-card"]')).map(m => m.getAttribute('data-signal'));
          const tools = document.querySelector('[data-component="signal-cell"][data-signal="unused_tools"] [data-component="signal-value"]')?.textContent?.trim() ?? null;
          return { headline: txt('[data-component="metrics-headline"]'), movers, tools, cells };
        })()
      `)) as { headline: string | null; movers: string[]; tools: string | null; cells: Array<{ key: string; moved: string; value?: string; delta: string | null }> };

      console.log(`\n[inspect] ${name} — headline: ${landing.headline ?? '(none)'}`);
      console.log(`[inspect] ${name} — movers: ${landing.movers.join(', ') || '(none)'}`);
      console.log(`[inspect] ${name} — tools cell (N of M): ${landing.tools ?? '(n/a)'}`);
      console.log(`[inspect] ${name} — ${landing.cells.length} signal cells:`);
      for (const c of landing.cells) {
        console.log(`  ${(c.value ?? '').padEnd(11)} ${(c.key ?? '').padEnd(24)} moved=${c.moved}${c.delta ? '  ' + c.delta : ''}`);
      }
      await shot(tauriPage, `${DIR}/metrics-${name}.png`);

      // ── Detail: drill into the first signal (a mover if any, else the first cell) ──
      const drillKey = landing.movers[0] ?? landing.cells[0]?.key;
      if (drillKey) {
        await navigateToScreen(
          tauriPage,
          `/workspace/${id}/metrics/${drillKey}`,
          '[data-component="signal-detail"]',
        );
        await settle();
        const insight = (await tauriPage.evaluate(
          `document.querySelector('[data-component="signal-insight"]')?.textContent?.replace(/\\s+/g,' ').trim() ?? null`,
        )) as string | null;
        console.log(`[inspect] ${name} — detail(${drillKey}) insight: ${insight ?? '(none)'}`);
        await shot(tauriPage, `${DIR}/metrics-${name}-detail.png`);
      }
    });
  }
});
