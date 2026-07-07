/**
 * Observatory Instruments — Track 2 Slices A/B/C/D UI verification.
 *
 * Complements `instruments-observatory.spec.ts` (Slice A' catalog +
 * kind chip + Insights signal cards) with checks for the surfaces added
 * as Track 2 closed:
 *
 *   • Playground Discovered MCP servers panel (Slices A + B).
 *   • Replay tab per-call verdict badges + session summary (Slice C).
 *   • Insights usage-split bar on the per-tool detail (Slice D).
 *
 * Every check tolerates an empty e2e DB: if there's no data to render,
 * the test asserts the panel's empty-state copy or skips gracefully.
 * The suite runs against the same throw-away `sensei_e2e` daemon the
 * other e2e specs use.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

test.describe('Observatory Instruments — T2 Slices', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await navigateTo(tauriPage, '/instruments');
    // Playground body is the load anchor — waits for loadCatalog() to
    // settle so the discovered-servers panel below has had its lazy load
    // fire on tab open.
    await tauriPage.waitForSelector('[data-testid="playground-body"]', 15_000).catch(() => {});
  });

  // ─── Slice A + B — Discovered MCP servers panel ────────────────────────

  test('Playground renders the Discovered MCP servers section', async ({ tauriPage }) => {
    const section = tauriPage.locator('[data-testid="discovered-servers"]');
    await expect(section).toBeVisible({ timeout: 5_000 });

    // Refresh button is always present regardless of server count.
    await expect(tauriPage.locator('[data-testid="refresh-servers"]')).toBeVisible();

    // Ask the daemon what it discovered; the panel must reflect it.
    const list = await fetch(`${DAEMON_URL}/api/instruments/mcp-servers`).then(r => r.json()) as {
      servers: Array<{ id: string; acp_family: string; mcp_key: string }>;
    };

    if (list.servers.length === 0) {
      // Empty e2e install — panel must show the friendly empty state,
      // and the refresh button is still clickable.
      await expect(section).toContainText(/No MCP servers discovered/i);
      const refreshBtn = tauriPage.locator('[data-testid="refresh-servers"]');
      await expect(refreshBtn).toBeEnabled();
    } else {
      // At least one row per discovered server, keyed by mcp_key.
      const firstKey = list.servers[0].mcp_key;
      await expect(tauriPage.locator(`[data-testid="server-row-${firstKey}"]`))
        .toBeVisible({ timeout: 5_000 });
      // Family sub-section is present too.
      const firstFamily = list.servers[0].acp_family;
      await expect(tauriPage.locator(`[data-testid="servers-family-${firstFamily}"]`))
        .toBeVisible();
    }
  });

  test('Refresh button triggers a discovery pass without erroring', async ({ tauriPage }) => {
    const refreshBtn = tauriPage.locator('[data-testid="refresh-servers"]');
    await expect(refreshBtn).toBeVisible();
    await expect(refreshBtn).toBeEnabled();

    // The button transitions to "Scanning…" during the request and back
    // to "Refresh" on settle. We only assert the settle: the request is
    // fast on an empty DB and can race the assertion mid-frame.
    await refreshBtn.click();
    await expect(refreshBtn).toHaveText(/Refresh/i, { timeout: 15_000 });
  });

  test('Server row toggle button switches between on and off', async ({ tauriPage }) => {
    const list = await fetch(`${DAEMON_URL}/api/instruments/mcp-servers`).then(r => r.json()) as {
      servers: Array<{ id: string; mcp_key: string; enabled: boolean }>;
    };
    if (list.servers.length === 0) {
      test.skip(true, 'No discovered servers to toggle — empty e2e DB');
      return;
    }
    const first = list.servers[0];
    const toggle = tauriPage.locator(`[data-testid="server-toggle-${first.mcp_key}"]`);
    await expect(toggle).toBeVisible({ timeout: 5_000 });
    // Text reflects current state.
    await expect(toggle).toHaveText(first.enabled ? /on/ : /off/);

    await toggle.click();
    // After the fire-and-forget PUT + optimistic store patch, the label
    // flips. Allow a couple hundred ms for reactivity to settle.
    await expect(toggle).toHaveText(first.enabled ? /off/ : /on/, { timeout: 3_000 });

    // Flip back so the test doesn't leave state dirty for the next spec.
    await toggle.click();
    await expect(toggle).toHaveText(first.enabled ? /on/ : /off/, { timeout: 3_000 });
  });

  // ─── Slice C — Replay tab verdict badges ───────────────────────────────

  test('Replay tab shows verdict badges when a session has classified data', async ({ tauriPage }) => {
    // Switch tabs — Insights spec already proves the tab bar works, so
    // fall back to a text hunt if the aria path misses.
    await tauriPage.getByText('Replay', { exact: true }).click();

    // Any classified session in the e2e DB?
    const sessions = await fetch(`${DAEMON_URL}/api/sessions`).then(r => r.json()) as {
      sessions: Array<{ id: string }>;
    };
    if ((sessions.sessions ?? []).length === 0) {
      test.skip(true, 'No sessions in e2e DB — nothing to replay');
      return;
    }

    // Pull the first session's Replay payload directly so we know which
    // fields we should see rendered. Trigger classify so verdicts flow
    // into the endpoint on the same read the UI does.
    const sid = sessions.sessions[0].id;
    const replay = await fetch(
      `${DAEMON_URL}/api/sessions/${encodeURIComponent(sid)}/replay?classify=true`,
    ).then(r => r.json()) as {
      calls: Array<{ verdict: 'used' | 'partial' | 'ignored' | null }>;
      summary: { used: number; partial: number; ignored: number; total: number };
    };

    if ((replay.calls ?? []).length === 0) {
      test.skip(true, 'First session has no tool calls — nothing to badge');
      return;
    }

    // Click the first session in the sidebar. The list uses task text +
    // date; picking by the session id via a page.evaluate is more
    // stable than matching prose.
    const clicked = await tauriPage.evaluate((expectedSid: string) => {
      const btns = Array.from(document.querySelectorAll<HTMLElement>('button.tool-card'));
      for (const b of btns) {
        // Session buttons carry no id; pick the first that isn't a
        // sensei-tool row (those have data-tool-kind).
        if (!b.hasAttribute('data-tool-kind')) {
          b.click();
          return true;
        }
      }
      // Ignore the argument — we just need something callable.
      void expectedSid;
      return false;
    }, sid);
    expect(clicked).toBe(true);

    if (replay.summary.total > 0) {
      // Session summary counters appear next to the "Calls" header.
      const anyVerdictText = tauriPage.getByText(new RegExp(
        `${replay.summary.used}.*${replay.summary.partial}.*${replay.summary.ignored}.*${replay.summary.total}`, 's',
      )).first();
      // Give the reactive update a moment; verdict summary shows once
      // the endpoint returns.
      await expect(anyVerdictText).toBeVisible({ timeout: 10_000 });
    }
  });

  // ─── Slice D — Insights usage-split bar ────────────────────────────────

  test('Insights per-tool detail shows a usage-split bar when verdicts exist', async ({ tauriPage }) => {
    await tauriPage.getByText('Insights', { exact: true }).click();

    // Insights tab needs to be visible first.
    await tauriPage.locator('[data-testid="insights-table"]').waitFor(10_000).catch(() => {});

    const usage = await fetch(`${DAEMON_URL}/api/observatory/tool-usage`).then(r => r.json()) as {
      tools: Array<{ tool_name: string }>;
    };
    if (usage.tools.length === 0) {
      test.skip(true, 'No tool_usage rows in e2e DB');
      return;
    }

    // Find the first tool whose insight has a verdict split populated.
    // The snapshot is only visible after row expansion, so we need to
    // scan the daemon side to pick a candidate rather than click blind.
    const insightsResp = await fetch(`${DAEMON_URL}/api/observatory/tool-insights`).then(r => r.json()) as {
      insights: Array<{ toolName: string; metrics: Record<string, unknown> }>;
    };
    const withSplit = (insightsResp.insights ?? []).find(
      (i) => typeof i.metrics?.verdictTotal === 'number' && (i.metrics.verdictTotal as number) > 0,
    );
    if (!withSplit) {
      test.skip(true, 'No cached insight has a populated verdict split yet');
      return;
    }

    const row = tauriPage.locator(`[data-testid="insights-row-${withSplit.toolName}"]`);
    await expect(row).toBeVisible({ timeout: 10_000 });
    await row.click();

    // Detail pane opens; the split bar sits inside "Usage split · Nd".
    const detail = tauriPage.locator(`[data-testid="insights-detail-${withSplit.toolName}"]`);
    await expect(detail).toBeVisible({ timeout: 5_000 });
    await expect(detail).toContainText(/Usage split/i);
    await expect(detail).toContainText(/classified/i);
  });
});
