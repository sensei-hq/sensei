/**
 * The e2e app and its daemon run with HOME set to a throwaway directory (#267).
 *
 * `SENSEI_INSTANCE=e2e` isolates the database and the data dir, but everything
 * the daemon does to an assistant resolves through HOME: configuring Claude Code
 * runs the real `claude plugin install` / `uninstall`, which edit
 * `~/.claude/settings.json` and `~/.claude/plugins/`. Under the developer's HOME
 * an e2e run uninstalled their own sensei plugin in its cleanup.
 *
 * The Keychain is reached the same way: router API keys and dōjō sessions are
 * stored under fixed service names, so a spec clearing the OpenAI key cleared the
 * developer's. The isolated home gets its own empty, unlocked keychain as its
 * default — `security` finds the default through HOME.
 *
 * Machine-wide installs the daemon reads through HOME are pointed back at the
 * real ones: the indexer's compiler engines and the Rust toolchain.
 */

import { execFileSync } from 'child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'fs';
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

  const keychain = join(E2E_HOME, 'Library', 'Keychains', 'e2e.keychain-db');
  mkdirSync(join(E2E_HOME, 'Library', 'Keychains'), { recursive: true });
  // `default-keychain` records the choice in Library/Preferences; without the
  // directory it records nothing, every Keychain call in the app finds no default,
  // and macOS raises "A keychain cannot be found" on the developer's screen.
  mkdirSync(join(E2E_HOME, 'Library', 'Preferences'), { recursive: true });
  const security = (...args: string[]) =>
    execFileSync('/usr/bin/security', args, { env: { ...process.env, HOME: E2E_HOME }, stdio: 'pipe' });
  security('create-keychain', '-p', '', keychain);
  security('set-keychain-settings', keychain); // no auto-lock mid-run
  security('unlock-keychain', '-p', '', keychain);
  security('list-keychains', '-d', 'user', '-s', keychain);
  security('default-keychain', '-d', 'user', '-s', keychain);
  const chosen = security('default-keychain', '-d', 'user').toString();
  if (!chosen.includes(keychain)) {
    throw new Error(`isolated home: default keychain is ${chosen.trim() || 'unset'}, not ${keychain}`);
  }

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

/** Whether the isolated home's keychain holds an item for `service`. */
export function e2eKeychainHas(service: string): boolean {
  try {
    execFileSync('/usr/bin/security', ['find-generic-password', '-s', service],
      { env: { ...process.env, HOME: E2E_HOME }, stdio: 'pipe' });
    return true;
  } catch {
    return false;
  }
}

/** Declare an MCP server in the isolated home's Cursor config, as a user who
 *  configured one would have. Discovery picks it up on its next scan. */
export function seedCursorMcpServer(key: string, command = '/usr/bin/true'): void {
  const dir = join(E2E_HOME, '.cursor');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'mcp.json'), JSON.stringify({ mcpServers: { [key]: { command, args: [] } } }));
}

/** The sensei-mcp the e2e daemon ships with (`make install-debug` overlays the brew prefix). */
export const SENSEI_MCP_BIN = '/opt/homebrew/bin/sensei-mcp';

/** Declare sensei's own MCP for Cursor in the isolated home, as `sensei init` writes it — the
 *  e2e home has no assistant configured with sensei until a spec does this. */
export function seedSenseiMcp(): void {
  seedCursorMcpServer('sensei', SENSEI_MCP_BIN);
}

/** Remove the Cursor config {@link seedCursorMcpServer} wrote. */
export function clearCursorMcpServers(): void {
  rmSync(join(E2E_HOME, '.cursor', 'mcp.json'), { force: true });
}
