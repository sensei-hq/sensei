/**
 * Setup wizard stage definitions — single source of truth for the rail,
 * the content header, and the wizard state.
 *
 * Icons are *meaning* kanji, not counters — the glyph reinforces what the
 * step is about, and survives reordering without going stale. The numeric
 * "01 / 11" lives in the bottom bar for a sense of progression.
 *
 * `brief` is the short rail copy (one sentence, tease voice).
 * `description` is the longer header tagline shown on the page.
 * `status` and `active` are live: `status` is persisted via wizardState's
 * commit handlers; `active` flips when the route changes.
 */

export type StageStatus = 'pending' | 'done';

export interface WizardStage {
  id: string;
  path: string;
  icon: string;
  title: string;
  brief: string;
  description: string;
  watermark: boolean;
  status: StageStatus;
  active: boolean;
}

export const STAGES: WizardStage[] = [
  {
    id: 'welcome',
    path: '/setup/welcome',
    icon: '礼',
    title: 'Welcome',
    brief: 'A quiet observer. Nothing more.',
    description: '',
    watermark: true,
    status: 'pending',
    active: false,
  },
  {
    id: 'preferences',
    path: '/setup/preferences',
    icon: '名',
    title: 'Preferences',
    brief: 'A few small choices before you step in.',
    description:
      'A few small choices before you step in. Anything here can be changed later by re-opening this wizard.',
    watermark: true,
    status: 'pending',
    active: false,
  },
  {
    id: 'assistants',
    path: '/setup/assistants',
    icon: '連',
    title: 'Assistants',
    brief: 'Connect the AI tools you already use.',
    description: 'Registers plugins, skills, commands, agents, logging and metrics.',
    watermark: true,
    status: 'pending',
    active: false,
  },
  {
    id: 'roots',
    path: '/setup/roots',
    icon: '庵',
    title: 'Roots',
    brief: 'Where your work lives.',
    description: 'Where your work lives. Sensei recurses and finds repos.',
    watermark: true,
    status: 'pending',
    active: false,
  },
  {
    id: 'scan',
    path: '/setup/scan',
    icon: '観',
    title: 'Scan',
    brief: 'Workers recurse. Repos surface.',
    description:
      'The daemon recurses your folders, identifies repos, and extracts the code graph.',
    watermark: false,
    status: 'pending',
    active: false,
  },
  {
    id: 'projects',
    path: '/setup/projects',
    icon: '組',
    title: 'Projects',
    brief: 'Each project, one or more repos.',
    description: 'A project has one or more repos. Edit, split, or confirm.',
    watermark: true,
    status: 'pending',
    active: false,
  },
  {
    id: 'libraries',
    path: '/setup/libraries',
    icon: '書',
    title: 'Libraries',
    brief: 'What sensei should wrap.',
    description:
      'Libraries without their own MCP — sensei indexes docs & code and wraps them with its own tools.',
    watermark: true,
    status: 'pending',
    active: false,
  },
  {
    id: 'instruments',
    path: '/setup/instruments',
    icon: '器',
    title: 'Instruments',
    brief: 'Tools sensei can reach for.',
    description:
      "Tools sensei can reach for — recommended based on what's in your stack. Each MCP brings its own capabilities.",
    watermark: true,
    status: 'pending',
    active: false,
  },
  {
    id: 'inference',
    path: '/setup/inference',
    icon: '想',
    title: 'Inference',
    brief: 'Local models, and a few clouds.',
    description:
      'Providers give sensei models for reasoning — inferring insights, consolidating memory, and making recommendations. Add providers, select models.',
    watermark: true,
    status: 'pending',
    active: false,
  },
  {
    id: 'done',
    path: '/setup/done',
    icon: '入',
    title: 'Enter',
    brief: 'The observatory is ready.',
    description: '',
    watermark: false,
    status: 'pending',
    active: false,
  },
];

/** Find the index of the current stage from a URL pathname. */
export function stageIndex(pathname: string): number {
  const segment = pathname.split('/').pop() ?? '';
  const idx = STAGES.findIndex((s) => s.id === segment);
  return idx >= 0 ? idx : 0;
}

/** Get next stage path, or null if at the end. */
export function nextStagePath(pathname: string): string | null {
  const idx = stageIndex(pathname);
  return idx < STAGES.length - 1 ? STAGES[idx + 1].path : null;
}

/** Get previous stage path, or null if at the start. */
export function prevStagePath(pathname: string): string | null {
  const idx = stageIndex(pathname);
  return idx > 0 ? STAGES[idx - 1].path : null;
}
