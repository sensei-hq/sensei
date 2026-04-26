// Setup wizard — types and stage definitions

export interface WizStage {
  id: string;
  n: string;        // kanji numeral (一, 二, ...)
  title: string;
  sub: string;      // subtitle shown in rail
  watermark: boolean; // show faded kanji watermark in content area
}

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

// ── Setup step types ────────────────────────────────────────

export interface AcpEntry {
  id: string;
  name: string;
  version: string | null;
  found: boolean;
  path: string | null;
}

export interface ScanFolder {
  id: string;
  path: string;
  note: string;
}

export interface ScanEvent {
  t: number;
  level: 'info' | 'discover' | 'queue' | 'process' | 'success';
  msg: string;
  parent?: string;
}

export interface DiscoveredProject {
  id: string;
  name: string;
  kanji: string;
  path: string;
  autoDetected: boolean;
  confidence: 'high' | 'medium' | 'low';
  repos: DiscoveredRepo[];
}

export interface DiscoveredRepo {
  id: string;
  name: string;
  path: string;
  files: number;
  lang: string;
  suggestedRole: string;
}

export interface DiscoveredLibrary {
  id: string;
  name: string;
  version: string;
  lang: string;
  usage: number;
  source: string;
  docs: 'indexed' | 'partial' | 'schema' | 'none';
  why: string;
}

export interface McpEntry {
  id: string;
  name: string;
  publisher: string;
  kind: 'data' | 'api' | 'devtool' | 'service';
  kanji: string;
  summary: string;
  trigger: string[];
  tools: number;
  verified: boolean;
  installed: boolean;
  recommended: boolean;
}

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

// ── Wizard accumulated state ────────────────────────────────

export interface WizardState {
  acps: Record<string, boolean>;
  acpList: AcpEntry[];
  folders: ScanFolder[];
  scanStarted: boolean;
  scanDone: boolean;
  scanTick: number;
  scanEvents: ScanEvent[];
  projects: (DiscoveredProject & { confirmed: boolean })[];
  roles: Record<string, string>;
  libraries: Record<string, boolean>;
  libExtras: { id: string; name: string; url: string }[];
  mcps: Record<string, boolean>;
  detectedStack: { languages: string[]; frameworks: string[]; runtimes: string[]; services: string[] };
}

export type WizUpdate = (patch: Partial<WizardState>) => void;
