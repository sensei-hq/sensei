// Setup wizard — types and stage definitions

export interface WizStage {
  id: string;
  n: string;     // kanji numeral (一, 二, ...)
  title: string;
  sub: string;   // subtitle shown in rail
}

export const WIZ_STAGES: WizStage[] = [
  { id: 'welcome',    n: '一', title: 'Welcome',      sub: 'a quiet observer of your work' },
  { id: 'components', n: '二', title: 'Components',   sub: 'installed automatically' },
  { id: 'assistants', n: '三', title: 'Assistants',   sub: 'plugins · skills · commands · logging' },
  { id: 'folders',    n: '四', title: 'Folders',      sub: 'where does your work live' },
  { id: 'scan',       n: '五', title: 'Scan',         sub: 'watching the worker' },
  { id: 'projects',   n: '六', title: 'Projects',     sub: 'one or more repos each' },
  { id: 'libraries',  n: '七', title: 'Libraries',    sub: 'what sensei should wrap' },
  { id: 'registry',   n: '八', title: 'MCP Registry', sub: 'recommended for your stack' },
  { id: 'done',       n: '九', title: 'Enter',        sub: 'the observatory is ready' },
];

// ── Component types ─────────────────────────────────────────

export interface ComponentStatus {
  id: string;
  name: string;
  version: string | null;
  status: 'missing' | 'installed' | 'stopped' | 'ready';
  icon: string; // short symbol: '$', '↔', '◇'
}

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
  components: ComponentStatus[];
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
