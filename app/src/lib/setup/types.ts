// Setup wizard — shared types not covered by contracts.ts.
// Daemon response types live in contracts.ts. Stage definitions live in stages.ts.

export interface RoleOption {
  id: string;
  label: string;
  kanji: string;
}

export const ROLES: RoleOption[] = [
  { id: 'backend',  label: 'Backend',  kanji: '後' },
  { id: 'frontend', label: 'Frontend', kanji: '前' },
  { id: 'library',  label: 'Library',  kanji: '書' },
  { id: 'docs',     label: 'Docs',     kanji: '記' },
  { id: 'infra',    label: 'Infra',    kanji: '基' },
];

// ── DEPRECATED — used by legacy /config page and setup/wizard/ components ──
// Remove when legacy single-page wizard is deleted.

/** @deprecated Use WizardStage from routes/(config)/stages.ts */
export interface WizStage {
  id: string;
  n: string;
  title: string;
  sub: string;
  watermark: boolean;
}

/** @deprecated Use WizardStage from routes/(config)/stages.ts */
export const WIZ_STAGES: WizStage[] = [
  { id: 'welcome',     n: '一', title: 'Welcome',     sub: 'a quiet observer of your work',       watermark: true  },
  { id: 'assistants',  n: '二', title: 'Assistants',  sub: 'plugins · skills · commands · logging', watermark: true  },
  { id: 'folders',     n: '三', title: 'Folders',     sub: 'where does your work live',            watermark: true  },
  { id: 'scan',        n: '四', title: 'Scan',        sub: 'watching the worker',                  watermark: false },
  { id: 'projects',    n: '五', title: 'Projects',    sub: 'one or more repos each',               watermark: true  },
  { id: 'libraries',   n: '六', title: 'Libraries',   sub: 'what sensei should wrap',              watermark: true  },
  { id: 'instruments', n: '七', title: 'Instruments', sub: 'recommended MCPs for your stack',      watermark: true  },
  { id: 'done',        n: '八', title: 'Enter',       sub: 'the observatory is ready',             watermark: false },
];

/** @deprecated Use contracts.ts types */
export interface AssistantEntry { id: string; name: string; version: string | null; found: boolean; path: string | null; }
/** @deprecated Use contracts.ts types */
export interface ScanFolder { id: string; path: string; note: string; }
/** @deprecated Use contracts.ts types */
export interface ScanEvent { t: number; level: 'info' | 'discover' | 'queue' | 'process' | 'success'; msg: string; parent?: string; }

/** @deprecated Use WizardState singleton from wizard-state.svelte.ts */
export interface WizardState {
  assistants: Record<string, boolean>;
  assistantList: AssistantEntry[];
  folders: ScanFolder[];
  scanStarted: boolean;
  scanDone: boolean;
  scanTick: number;
  scanEvents: ScanEvent[];
  projects: any[];
  roles: Record<string, string>;
  libraries: Record<string, boolean>;
  libExtras: { id: string; name: string; url: string }[];
  mcps: Record<string, boolean>;
  detectedStack: { languages: string[]; frameworks: string[]; runtimes: string[]; services: string[] };
}

/** @deprecated */
export type WizUpdate = (patch: Partial<WizardState>) => void;

/** @deprecated Use DaemonProject from contracts.ts */
export interface DiscoveredProject {
  id: string; name: string; kanji: string; path: string;
  autoDetected: boolean; confidence: 'high' | 'medium' | 'low';
  repos: { id: string; name: string; path: string; files: number; lang: string; suggestedRole: string }[];
}

/** @deprecated */
export type DiscoveredRepo = DiscoveredProject['repos'][number];

/** @deprecated Use DaemonLibEntry from contracts.ts */
export interface DiscoveredLibrary {
  id: string; name: string; version: string; lang: string;
  usage: number; source: string; docs: 'indexed' | 'partial' | 'schema' | 'none'; why: string;
}

/** @deprecated Use DaemonMcpEntry from contracts.ts */
export interface McpEntry {
  id: string; name: string; publisher: string; kind: 'data' | 'api' | 'devtool' | 'service';
  kanji: string; summary: string; trigger: string[]; tools: number;
  verified: boolean; installed: boolean; recommended: boolean;
}
