/**
 * Mock factory functions for daemon contracts.
 * Each function returns one valid instance with sensible defaults.
 * Pass overrides to customize specific fields.
 */

import type {
  DaemonWatchRoot, DaemonAssistantFamily, DaemonProject,
  DaemonLibEntry, DaemonMcpEntry, PreferencesData, WizardLoadData,
} from './contracts.js';

export function mockWatchRoot(overrides: Partial<DaemonWatchRoot> = {}): DaemonWatchRoot {
  return {
    id: 'root-1', path: '/Users/test/code', name: 'Code',
    status: 'watching', excluded: ['node_modules', '.git'],
    repos_found: 3, scanned: true, modified_at: '2026-04-30T00:00:00Z',
    ...overrides,
  };
}

export function mockAssistant(overrides: Partial<DaemonAssistantFamily> = {}): DaemonAssistantFamily {
  return {
    id: 'claude-code', name: 'Claude Code', installed: true, selected: true,
    config_path: '~/.claude/config.json', version: '1.0.0', install_path: '/usr/local/bin/claude',
    ...overrides,
  };
}

export function mockProject(overrides: Partial<DaemonProject> = {}): DaemonProject {
  return {
    id: 'proj-1', name: 'Test Project', description: null, client: null, goal: null,
    stack: { languages: ['TypeScript'], frameworks: ['SvelteKit'], runtimes: ['Node 20'], services: [] },
    icon: { kind: 'kanji', value: '工' },
    folders: [
      { id: 'f-1', name: 'app', path: '/Users/test/code/app', kind: 'git', role: 'frontend' },
    ],
    ...overrides,
  };
}

export function mockLibEntry(overrides: Partial<DaemonLibEntry> = {}): DaemonLibEntry {
  return {
    id: 'svelte', name: 'svelte', version: '5.0.0', lang: 'TypeScript',
    usage: 42, source: 'package.json', docs: 'indexed', enabled: true,
    ...overrides,
  };
}

export function mockMcpEntry(overrides: Partial<DaemonMcpEntry> = {}): DaemonMcpEntry {
  return {
    id: 'postgres-mcp', name: 'PostgreSQL MCP', publisher: 'supabase', kind: 'data',
    summary: 'Query schema, introspect tables.', tools: 14, verified: true,
    installed: false, recommended: true, selected: true, project_count: 2,
    ...overrides,
  };
}

export function mockPreferences(overrides: Partial<PreferencesData> = {}): PreferencesData {
  return {
    displayName: 'Jerry', contributeLearnings: true, reviewBeforeShare: true,
    shareSchedule: 'weekly-saturday', downloadCollective: 'weekly',
    correctionAggressiveness: 'balanced', digestCadence: 'daily',
    nudgeOnRegression: true, anonymizedTelemetry: false, showWelcome: true,
    ...overrides,
  };
}

export function mockWizardLoadData(overrides: Partial<WizardLoadData> = {}): WizardLoadData {
  return {
    completion: {
      welcome: 'pending', preferences: 'pending', assistants: 'pending',
      roots: 'pending', scan: 'pending', projects: 'pending',
      libraries: 'pending', instruments: 'pending',
      inference: 'pending', assignments: 'pending', done: 'pending',
    },
    preferences: mockPreferences(),
    assistantFamilies: [
      mockAssistant(),
      mockAssistant({ id: 'cursor', name: 'Cursor', installed: false, selected: false }),
    ],
    roots: [mockWatchRoot()],
    projects: [mockProject()],
    libraries: { total: 1, libs: [mockLibEntry()] },
    mcps: [mockMcpEntry()],
    ...overrides,
  };
}
