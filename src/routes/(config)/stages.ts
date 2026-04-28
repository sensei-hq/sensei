/**
 * Setup wizard stage definitions — metadata for the rail and navigation.
 * Each stage maps to a route under (config)/.
 */

export interface WizardStage {
  id: string;
  path: string;
  icon: string;
  title: string;
  description: string;
  watermark: boolean;
}

export const STAGES: WizardStage[] = [
  { id: 'welcome',     path: '/setup/welcome',     icon: '一', title: 'Welcome',     description: 'a quiet observer of your work',          watermark: true  },
  { id: 'assistants',  path: '/setup/assistants',   icon: '二', title: 'Assistants',  description: 'plugins, skills, commands, logging',      watermark: true  },
  { id: 'folders',     path: '/setup/folders',      icon: '三', title: 'Folders',     description: 'where does your work live',               watermark: true  },
  { id: 'scan',        path: '/setup/scan',         icon: '四', title: 'Scan',        description: 'watching the worker',                     watermark: false },
  { id: 'projects',    path: '/setup/projects',     icon: '五', title: 'Projects',    description: 'one or more repos each',                  watermark: true  },
  { id: 'libraries',   path: '/setup/libraries',    icon: '六', title: 'Libraries',   description: 'what sensei should know',                 watermark: true  },
  { id: 'instruments', path: '/setup/instruments',  icon: '七', title: 'Instruments', description: 'recommended MCPs for your stack',         watermark: true  },
  { id: 'done',        path: '/setup/done',         icon: '八', title: 'Enter',       description: 'the observatory is ready',                watermark: false },
];

/** Find the index of the current stage from a URL pathname. */
export function stageIndex(pathname: string): number {
  const segment = pathname.split('/').pop() ?? '';
  const idx = STAGES.findIndex(s => s.id === segment);
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
