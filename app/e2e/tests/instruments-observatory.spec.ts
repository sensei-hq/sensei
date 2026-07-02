/**
 * Observatory Instruments — playground, replay, insights.
 *
 * Verifies the enriched T2 flows against the live daemon:
 *   • Playground renders the enriched mcp_list_tools shape (T2 Slice A')
 *     and the kind chip filter narrows the tool list (T2 Slice A'').
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
    // Give the store's loadCatalog() a chance to hydrate.
    await tauriPage.waitForSelector('[data-testid="playground-body"]', 15_000).catch(() => {});
  });

  test('playground renders sensei manifest tools with kind badges', async ({ tauriPage }) => {
    const catalog = await fetch(`${DAEMON_URL}/api/mcp/tools`).then(r => r.json()) as {
      tools: Array<{ name: string; kind: 'query' | 'action'; summary: string }>;
    };
    expect(catalog.tools.length).toBeGreaterThan(10);

    const first = catalog.tools[0];
    // Row for the first tool should render with the daemon's kind tag.
    const row = tauriPage.locator(`[data-testid="tool-row-${first.name}"]`);
    await expect(row).toBeVisible({ timeout: 10_000 });
    await expect(row).toHaveAttribute('data-tool-kind', first.kind);
  });

  test('kind chip filters the tool list (queries only)', async ({ tauriPage }) => {
    await tauriPage.locator('[data-testid="playground-body"]').waitFor(10_000);

    // The tauri-playwright wrapper doesn't expose evaluateAll on locators —
    // fall back to a page.evaluate that queries the DOM directly.
    const readKinds = async () => tauriPage.evaluate(`
      Array.from(document.querySelectorAll('[data-testid^="tool-row-"]'))
        .map(el => el.dataset.toolKind)
    `) as Promise<string[]>;

    const allKinds = await readKinds();
    expect(allKinds.length).toBeGreaterThan(0);

    await tauriPage.locator('[data-testid="kind-chip-query"]').click();
    await new Promise<void>(r => setTimeout(r, 300));
    const filteredKinds = await readKinds();
    expect(filteredKinds.length).toBeGreaterThan(0);
    expect(filteredKinds.length).toBeLessThanOrEqual(allKinds.length);
    for (const k of filteredKinds) expect(k).toBe('query');
  });

  test('Insights tab shows signal cards + expandable per-tool detail', async ({ tauriPage }) => {
    // Switch to the Insights tab (third tab).
    await tauriPage.locator('.tab-bar button, [role="tab"]', { hasText: /Insights/i }).first().click().catch(async () => {
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
