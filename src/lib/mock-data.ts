/**
 * Mock data for test mode — allows running all pages in the browser
 * without daemon or Tauri sidecar.
 *
 * Enable: set localStorage.setItem('sensei:test-mode', '1')
 * Disable: localStorage.removeItem('sensei:test-mode')
 */

import type { BootstrapResult, ComponentStatus } from './bootstrap.js';

export function isTestMode(): boolean {
  if (typeof localStorage === 'undefined') return false;
  return localStorage.getItem('sensei:test-mode') === '1';
}

// ── Bootstrap mock data ──────────────────────────────────────────────────────

export const mockBootstrapHealthy: BootstrapResult = {
  components: [
    { name: 'Homebrew', state: { state: 'ready' }, version: '4.4.2', detail: null },
    { name: 'sensei', state: { state: 'ready' }, version: '0.9.4', detail: null },
    { name: 'PostgreSQL', state: { state: 'ready' }, version: '17.2', detail: null },
    { name: 'Ollama', state: { state: 'ready' }, version: '0.6.2', detail: 'gemma3:27b · qwen3:14b' },
    { name: 'Database', state: { state: 'ready' }, version: 'schema-42', detail: null },
    { name: 'Daemon', state: { state: 'ready' }, version: '0.9.4', detail: null },
  ],
  hardware: { ram_gb: 32, cpu_cores: 12, gpu: 'Apple M2 Pro', metal_support: true, recommended_tier: 'full' },
  ready: true,
};

export const mockBootstrapPartial: BootstrapResult = {
  components: [
    { name: 'Homebrew', state: { state: 'ready' }, version: '4.4.2', detail: null },
    { name: 'sensei', state: { state: 'ready' }, version: '0.9.4', detail: null },
    { name: 'PostgreSQL', state: { state: 'ready' }, version: '17.2', detail: null },
    { name: 'Ollama', state: { state: 'failed', error: 'not running' }, version: null, detail: null },
    { name: 'Database', state: { state: 'ready' }, version: 'schema-42', detail: null },
    { name: 'Daemon', state: { state: 'failed', error: 'not reachable on port 7744' }, version: null, detail: null },
  ],
  hardware: { ram_gb: 16, cpu_cores: 8, gpu: 'Apple M2', metal_support: true, recommended_tier: 'recommended' },
  ready: false,
};

export const mockBootstrapFresh: BootstrapResult = {
  components: [
    { name: 'Homebrew', state: { state: 'failed', error: 'not installed' }, version: null, detail: null },
    { name: 'sensei', state: { state: 'failed', error: 'homebrew not installed' }, version: null, detail: null },
    { name: 'PostgreSQL', state: { state: 'failed', error: 'not installed' }, version: null, detail: null },
    { name: 'Ollama', state: { state: 'failed', error: 'not installed' }, version: null, detail: null },
    { name: 'Database', state: { state: 'failed', error: 'postgresql not reachable' }, version: null, detail: null },
    { name: 'Daemon', state: { state: 'failed', error: 'not reachable on port 7744' }, version: null, detail: null },
  ],
  hardware: { ram_gb: 16, cpu_cores: 8, gpu: null, metal_support: false, recommended_tier: 'recommended' },
  ready: false,
};

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
