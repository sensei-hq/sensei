/**
 * Mock data for test mode — allows running all pages in the browser
 * without daemon or Tauri sidecar.
 *
 * Enable: set localStorage.setItem(STORAGE_KEYS.testMode, '1')  // e.g. 'sensei-dev:test-mode'
 * Disable: localStorage.removeItem(STORAGE_KEYS.testMode)
 */

import { STORAGE_KEYS } from './storage-keys.js';

export function isTestMode(): boolean {
  if (typeof window === 'undefined') return false;
  // URL param ?test=1 enables and persists test mode
  if (typeof URLSearchParams !== 'undefined') {
    const params = new URLSearchParams(window.location.search);
    if (params.get('test') === '1') {
      localStorage.setItem(STORAGE_KEYS.testMode, '1');
      return true;
    }
  }
  return localStorage.getItem(STORAGE_KEYS.testMode) === '1';
}

// ── Assistants mock data ─────────────────────────────────────────────────────

export const mockAssistants = [
  { id: 'claude-code', name: 'Claude Code', installed: true, configPath: '~/.claude/settings.json' },
  { id: 'cursor', name: 'Cursor', installed: true, configPath: '~/.cursor/config.json' },
  { id: 'windsurf', name: 'Windsurf', installed: false, configPath: null },
  { id: 'copilot', name: 'GitHub Copilot', installed: false, configPath: null },
];

// ── Folders mock data ────────────────────────────────────────────────────────

export const mockFolders = [
  { id: 'r1', path: '~/Developer', note: '12 folders found', watched: true },
  { id: 'r2', path: '~/Work', note: '3 folders found', watched: true },
];
