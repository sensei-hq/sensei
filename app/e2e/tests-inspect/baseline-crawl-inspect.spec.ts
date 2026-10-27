/**
 * BASELINE CRAWL (user, 2026-10-27): every screen of the desktop app against the
 * REAL daemon — does it render, does it log a console error, and does every
 * clickable element on it work (no error, no "Not found", no blank page)?
 *
 * One test per route; results append to /tmp/sensei-baseline.jsonl as they go,
 * so a slow or failing route never loses the others. Nothing that changes data
 * is clicked: destructive or outward verbs are skipped, switches and
 * checkboxes are never toggled, and on settings / setup / dōjō-connection
 * screens only links and tabs are followed.
 *
 * Run: `bun run test:inspect -- baseline-crawl` (long: a few seconds per click).
 */
import { readdirSync, statSync, appendFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from '../fixtures';
import { DAEMON_URL, navigateTo, takeConsoleErrors, watchConsole } from '../helpers';

const OUT = '/tmp/sensei-baseline.jsonl';
const SKIP_LABEL =
  /delete|remove|sign ?out|log ?out|disable|enable|reset|uninstall|revoke|discard|merge|archive|clear|exclude|drop|prune|re-?scan|scan|install|upgrade|publish|approve|reject|accept|send|share|connect|disconnect|forget|leave|invite|create|save|apply|run|execute|query →|run →|retry|refresh|classify|import|export|sync|add/i;
const LINKS_ONLY = /^\/(settings|setup|dojo|share-review|upgrade)/;
const PER_ROUTE_CAP = 30;

function routes(): string[] {
  const out: string[] = [];
  const walk = (dir: string) => {
    for (const name of readdirSync(dir)) {
      const p = join(dir, name);
      if (statSync(p).isDirectory()) walk(p);
      else if (name === '+page.svelte') {
        const route = p
          .replace(/^src\/routes/, '')
          .replace(/\/\+page\.svelte$/, '')
          .replace(/\/\([^)]+\)/g, '');
        out.push(route || '/');
      }
    }
  };
  walk('src/routes');
  return [...new Set(out)].sort();
}

const ALL = routes();
writeFileSync(OUT, '');

let wsId = '';
let metricKey = '';

const PROBE = `JSON.stringify({
  path: location.pathname + location.search,
  text: (document.querySelector('main') || document.body).innerText.trim().length,
  notFound: /\\bNot found\\b|\\b404\\b/.test(document.body.innerText.slice(0, 400)),
  errorStates: [...document.querySelectorAll('[data-screen-state="error"], [data-testid$="-error"], .text-danger, .text-error')]
    .map((e) => e.innerText.trim()).filter(Boolean).slice(0, 5),
})`;

const CLICKABLES = (linksOnly: boolean) => `JSON.stringify((() => {
  const root = document.querySelector('main') || document.body;
  const sel = ${linksOnly} ? 'a[href], [role=tab]' : 'a[href], button, [role=tab], [role=button], [role=menuitem], summary';
  const seen = new Set();
  return [...root.querySelectorAll(sel)].filter((e) => {
    const r = e.getBoundingClientRect();
    if (!r.width || !r.height || e.disabled || e.getAttribute('aria-disabled') === 'true') return false;
    if (e.getAttribute('role') === 'switch' || e.closest('[role=dialog]')) return false;
    const href = e.getAttribute('href') || '';
    if (/^(https?:|mailto:)/.test(href)) return false;
    const label = (e.getAttribute('data-testid') || e.getAttribute('aria-label') || e.innerText || '').trim().slice(0, 60);
    const key = e.tagName + '|' + label + '|' + href;
    if (!label || seen.has(key)) return false;
    seen.add(key);
    return true;
  }).map((e, i) => {
    e.setAttribute('data-crawl', String(i));
    return { i, tag: e.tagName.toLowerCase(), label: (e.getAttribute('data-testid') || e.getAttribute('aria-label') || e.innerText || '').trim().replace(/\\s+/g, ' ').slice(0, 60), href: e.getAttribute('href') };
  });
})())`;

test.describe.configure({ mode: 'serial' });

test.beforeAll(async () => {
  const workspaces = await (await fetch(`${DAEMON_URL}/api/workspaces`)).json();
  const rows = Array.isArray(workspaces) ? workspaces : workspaces.workspaces;
  wsId = (rows.find((w: { name: string }) => w.name === 'sensei') ?? rows[0]).id;
  const registry = await (await fetch(`${DAEMON_URL}/api/metrics/registry`)).json();
  metricKey = registry.metrics.find((m: { key: string }) => m.key === 'ftr')?.key ?? registry.metrics[0].key;
});

for (const pattern of ALL) {
  test(`crawl ${pattern}`, async ({ tauriPage }) => {
    test.setTimeout(10 * 60_000);
    const route = pattern.replace('[id]', wsId).replace('[key]', metricKey);
    await watchConsole(tauriPage);
    await takeConsoleErrors(tauriPage);
    await navigateTo(tauriPage, route);
    await new Promise((r) => setTimeout(r, 2500));
    const page = JSON.parse((await tauriPage.evaluate(PROBE)) as string);
    const errors = await takeConsoleErrors(tauriPage);
    const clicks: unknown[] = [];

    const linksOnly = LINKS_ONLY.test(route);
    const list = JSON.parse((await tauriPage.evaluate(CLICKABLES(linksOnly))) as string) as
      { i: number; tag: string; label: string; href: string | null }[];
    for (const c of list.filter((c) => !SKIP_LABEL.test(c.label)).slice(0, PER_ROUTE_CAP)) {
      // Back on the route, re-marked, so each click starts from the page as loaded.
      const here = (await tauriPage.evaluate('location.pathname + location.search')) as string;
      if (here !== route) {
        await navigateTo(tauriPage, route);
        await new Promise((r) => setTimeout(r, 1500));
      }
      await tauriPage.evaluate(CLICKABLES(linksOnly));
      await takeConsoleErrors(tauriPage);
      const found = (await tauriPage.evaluate(
        `(() => { const e = [...document.querySelectorAll('[data-crawl]')].find((x) => (x.getAttribute('data-testid') || x.getAttribute('aria-label') || x.innerText || '').trim().replace(/\\s+/g, ' ').slice(0, 60) === ${JSON.stringify(c.label)}); if (!e) return false; e.click(); return true; })()`,
      )) as boolean;
      await new Promise((r) => setTimeout(r, 1800));
      const after = JSON.parse((await tauriPage.evaluate(PROBE)) as string);
      const errs = await takeConsoleErrors(tauriPage);
      await tauriPage.evaluate(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))`);
      clicks.push({ ...c, found, after, errors: errs });
    }
    appendFileSync(OUT, JSON.stringify({ route: pattern, page, errors, clickables: list.length, clicks }) + '\n');
  });
}
