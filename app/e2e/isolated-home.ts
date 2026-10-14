/**
 * The e2e app and its daemon run with HOME set to a throwaway directory (#267).
 *
 * `SENSEI_INSTANCE=e2e` isolates the database and the data dir, but everything
 * the daemon does to an assistant resolves through HOME: configuring Claude Code
 * runs the real `claude plugin install` / `uninstall`, which edit
 * `~/.claude/settings.json` and `~/.claude/plugins/`. Under the developer's HOME
 * an e2e run uninstalled their own sensei plugin in its cleanup.
 *
 * Machine-wide installs the daemon reads through HOME are pointed back at the
 * real ones: the indexer's compiler engines and the Rust toolchain.
 */

import { existsSync, mkdirSync, readFileSync, rmSync, symlinkSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';

/** HOME for the e2e app, its daemon, and every assistant CLI the daemon runs. */
export const E2E_HOME = '/tmp/sensei-e2e-home';

/** Claude Code's settings and plugin manifest inside the isolated home. */
export const E2E_CLAUDE_SETTINGS = join(E2E_HOME, '.claude', 'settings.json');
export const E2E_CLAUDE_PLUGINS = join(E2E_HOME, '.claude', 'plugins', 'installed_plugins.json');

/**
 * A fresh isolated home, and the env that puts the app into it. `.claude/`
 * exists so Claude Code is detected, as it is on a machine that has it.
 */
export function isolatedHomeEnv(): Record<string, string> {
  const real = homedir();
  rmSync(E2E_HOME, { recursive: true, force: true });
  mkdirSync(join(E2E_HOME, '.claude'), { recursive: true });
  mkdirSync(join(E2E_HOME, '.sensei'), { recursive: true });

  const engines = join(real, '.sensei', 'engines');
  if (existsSync(engines)) symlinkSync(engines, join(E2E_HOME, '.sensei', 'engines'));

  return {
    HOME: E2E_HOME,
    RUSTUP_HOME: process.env.RUSTUP_HOME ?? join(real, '.rustup'),
    CARGO_HOME: process.env.CARGO_HOME ?? join(real, '.cargo'),
  };
}

function readJson(path: string): Record<string, unknown> | null {
  if (!existsSync(path)) return null;
  return JSON.parse(readFileSync(path, 'utf8'));
}

/** How many `sensei@…` installs Claude Code's manifest records in the isolated home. */
export function senseiPluginInstalls(): number {
  const plugins = (readJson(E2E_CLAUDE_PLUGINS)?.plugins ?? {}) as Record<string, unknown[]>;
  return Object.entries(plugins)
    .filter(([key]) => key.startsWith('sensei@'))
    .reduce((n, [, entries]) => n + entries.length, 0);
}

/** Whether the isolated home's settings enable the sensei plugin. */
export function senseiPluginEnabled(): boolean {
  const enabled = (readJson(E2E_CLAUDE_SETTINGS)?.enabledPlugins ?? {}) as Record<string, unknown>;
  return enabled['sensei@sensei-marketplace'] === true;
}
