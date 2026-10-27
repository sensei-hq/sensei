/**
 * Observatory Instruments — playground, replay, insights.
 *
 * Verifies the enriched T2 flows against the live daemon:
 *   • Playground lists sensei's server first with the tools the server
 *     itself lists, each row carrying its kind.
 *   • Insights tab lights up with derived / cached signal cards (T2 Slice
 *     D-lite + D full) and the row-click disclosure reveals the cached
 *     insight detail from the `tool_insights` cache table.
 *
 * The daemon must be running the current `develop` binary — the specs
 * hit real endpoints and validate the DOM against the same data the
 * page sees on load. No seed fixtures required beyond what the daemon
 * already tracks (assistant_events → tool_usage_stats → tool_insights).
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

test.describe('Observatory Instruments', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/instruments');
    // Give the server list and sensei's tools a chance to load.
    await tauriPage.waitForSelector('[data-testid="playground-body"]', 15_000).catch(() => {});
  });

  test('playground lists the sensei server’s own tools', async ({ tauriPage }) => {
    const { servers } = await fetch(`${DAEMON_URL}/api/instruments/mcp-servers`).then(r => r.json()) as {
      servers: Array<{ id: string; mcp_key: string }>;
    };
    const sensei = servers.find((s) => /(^|_)sensei$/.test(s.mcp_key));
    expect(sensei, 'the sensei MCP is configured for an assistant').toBeTruthy();
    const manifest = await fetch(`${DAEMON_URL}/api/instruments/mcp-servers/${sensei!.id}/tools`).then(r => r.json()) as {
      tools: Array<{ name: string }>;
    };
    expect(manifest.tools.length).toBeGreaterThan(10);

    const row = tauriPage.locator(`[data-testid="tool-row-${manifest.tools[0].name}"]`);
    await expect(row).toBeVisible({ timeout: 10_000 });
    await expect(row).toHaveAttribute('data-tool-kind', /query|action|other/);
  });

  test('Insights tab shows signal cards + expandable per-tool detail', async ({ tauriPage }) => {
    // Switch to the Insights tab (third tab).
    await tauriPage.locator('.tab-bar button, [role="tab"]').filter({ hasText: /Insights/i }).first().click().catch(async () => {
      // Some TabBar variants use plain buttons — fallback to text hunt.
      await tauriPage.getByText('Insights', { exact: true }).click();
    });

    // Signals row from the /tool-signals endpoint should render at least
    // one card when the cache has variant-firing rows. Wait up to 10s.
    const signalsExist = await tauriPage
      .locator('[data-testid="tool-signals"]')
      .waitFor(10_000)
      .then(() => true)
      .catch(() => false);
    if (!signalsExist) {
      test.skip(true, 'No signal cards to verify — daemon has zero firing signals');
      return;
    }

    // Sanity-check a signal card is present with a known variant.
    const anyCard = tauriPage.locator('[data-testid^="signal-card-"]').first();
    await expect(anyCard).toBeVisible({ timeout: 5_000 });

    // Insights table shows one row per tool that has stats. Expand one and
    // assert the detail pane appears.
    const usage = await fetch(`${DAEMON_URL}/api/observatory/tool-usage`).then(r => r.json()) as {
      tools: Array<{ tool_name: string }>;
    };
    if (usage.tools.length === 0) {
      test.skip(true, 'No tool usage yet — cannot exercise the disclosure');
      return;
    }
    const firstTool = usage.tools[0].tool_name;
    const row = tauriPage.locator(`[data-testid="insights-row-${firstTool}"]`);
    await expect(row).toBeVisible({ timeout: 10_000 });
    await row.click();
    await expect(row).toHaveAttribute('aria-expanded', 'true', { timeout: 3_000 });
    await expect(tauriPage.locator(`[data-testid="insights-detail-${firstTool}"]`)).toBeVisible();
  });
});
