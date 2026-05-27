/**
 * Mock factory functions for daemon contracts.
 * Each function returns one valid instance with sensible defaults.
 * Pass overrides to customize specific fields.
 */

import type {
  DaemonWatchRoot, DaemonAssistantFamily, AssistantVariant, DaemonProject,
  DaemonLibEntry, DaemonMcpEntry, DaemonRouter, PreferencesData, WizardLoadData,
  Memory, MemoryDetail,
} from './contracts.js';

export function mockWatchRoot(overrides: Partial<DaemonWatchRoot> = {}): DaemonWatchRoot {
  return {
    id: 'root-1', path: '/Users/test/code', name: 'Code',
    status: 'watching', excluded: ['node_modules', '.git'],
    repos_found: 3, scanned: true, modified_at: '2026-04-30T00:00:00Z',
    ...overrides,
  };
}

export function mockAssistantVariant(overrides: Partial<AssistantVariant> = {}): AssistantVariant {
  return {
    id: 'claude-code', name: 'Claude Code', installed: true, configured: false,
    ...overrides,
  };
}

export function mockAssistant(overrides: Partial<DaemonAssistantFamily> = {}): DaemonAssistantFamily {
  return {
    id: 'claude',
    name: 'Claude',
    selected: true,
    variants: [
      mockAssistantVariant(),
      mockAssistantVariant({ id: 'claude-desktop', name: 'Claude Desktop' }),
    ],
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
    id: 'svelte', name: 'svelte', ecosystem: 'npm', version: '5.0.0',
    description: 'Cybernetically enhanced web apps', pageCount: 0,
    repos: ['app'], repoCount: 1, enabled: true,
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

export function mockRouter(overrides: Partial<DaemonRouter> = {}): DaemonRouter {
  return {
    id: 'openai',
    name: 'OpenAI',
    providers: ['openai'],
    capabilities: ['text_chat', 'text_embed', 'image_generate'],
    needs_key: true,
    configured: false,
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
      mockAssistant({
        id: 'cursor', name: 'Cursor', selected: false,
        variants: [mockAssistantVariant({ id: 'cursor', name: 'Cursor', installed: false })],
      }),
    ],
    roots: [mockWatchRoot()],
    projects: [mockProject()],
    libraries: { total: 1, libs: [mockLibEntry()] },
    mcps: [mockMcpEntry()],
    routers: [mockRouter()],
    ...overrides,
  };
}

export function mockMemory(overrides: Partial<Memory> = {}): Memory {
  return {
    id: 'mem-1', project_id: 'proj-1', scope: 'project', scope_filter: null,
    type: 'convention', title: 'Test memory', content: 'body',
    impact: null, strength: 1.0, status: 'proposed',
    applied_count: 0, violated_count: 0, last_relevant_at: null,
    tags: [], triage_signal: 'revert',
    modified_at: '2026-05-27T00:00:00Z',
    ...overrides,
  };
}

export function mockMemoryDetail(overrides: Partial<MemoryDetail> = {}): MemoryDetail {
  return {
    memory: mockMemory(),
    evidence: [],
    examples: [],
    outcomes: [],
    ...overrides,
  };
}
