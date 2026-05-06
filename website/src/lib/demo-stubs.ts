export const demoUser = { name: 'Jerry Thomas', email: 'jerry@acme.com', initials: 'JT' };

export const demoWorkspaces = [
  { id: 'personal', name: 'Jerry Thomas', type: 'personal' as const },
  { id: 'acme', name: 'Acme Corp', type: 'org' as const },
];

export const demoRepos = [
  { id: 'r1', name: 'sensei',    workspaceId: 'personal', lang: 'TypeScript', indexed: '2h ago',  symbols: 1847, sessions: 24, status: 'ready'    as const },
  { id: 'r2', name: 'rokkit',    workspaceId: 'personal', lang: 'TypeScript', indexed: '1d ago',  symbols: 923,  sessions: 8,  status: 'ready'    as const },
  { id: 'r3', name: 'kavach',    workspaceId: 'personal', lang: 'TypeScript', indexed: '3d ago',  symbols: 412,  sessions: 3,  status: 'indexing' as const },
  { id: 'r4', name: 'acme-api',  workspaceId: 'acme',     lang: 'Go',         indexed: '5h ago',  symbols: 2100, sessions: 15, status: 'ready'    as const },
  { id: 'r5', name: 'acme-web',  workspaceId: 'acme',     lang: 'TypeScript', indexed: '1d ago',  symbols: 675,  sessions: 6,  status: 'ready'    as const },
];

export const demoTeams = [
  { id: 't1', name: 'Backend',  workspaceId: 'acme', memberCount: 5, repoIds: ['r4'] },
  { id: 't2', name: 'Frontend', workspaceId: 'acme', memberCount: 3, repoIds: ['r5'] },
];

export const demoLibraries = [
  { id: 'l1', name: 'Rokkit UI',   category: 'ui'   as const, sections: 142, documents: 28 },
  { id: 'l2', name: 'Kavach Auth', category: 'auth' as const, sections: 67,  documents: 12 },
  { id: 'l3', name: 'Supabase JS', category: 'data' as const, sections: 215, documents: 41 },
];

export const demoSessions = [
  { id: 's1', repoId: 'r1', task: 'Fix OTLP telemetry pipeline',   duration: '42m',    ftr: 0.87, when: '2h ago',  model: 'sonnet-4-6', turns: 12, cost: 0.14 },
  { id: 's2', repoId: 'r1', task: 'Add SessionStart hook',          duration: '18m',    ftr: 1.00, when: '3h ago',  model: 'sonnet-4-6', turns: 5,  cost: 0.04 },
  { id: 's3', repoId: 'r4', task: 'Implement auth middleware',       duration: '1h 5m',  ftr: 0.72, when: '5h ago',  model: 'sonnet-4-6', turns: 18, cost: 0.22 },
  { id: 's4', repoId: 'r2', task: 'Design token system',            duration: '28m',    ftr: 0.95, when: '1d ago',  model: 'haiku-4-5',  turns: 7,  cost: 0.02 },
  { id: 's5', repoId: 'r1', task: 'Dashboard analytics page',       duration: '55m',    ftr: 0.81, when: '1d ago',  model: 'sonnet-4-6', turns: 14, cost: 0.18 },
  { id: 's6', repoId: 'r3', task: 'OAuth flow implementation',      duration: '35m',    ftr: 0.92, when: '2d ago',  model: 'sonnet-4-6', turns: 9,  cost: 0.09 },
  { id: 's7', repoId: 'r5', task: 'Build dashboard layout',         duration: '47m',    ftr: 0.78, when: '2d ago',  model: 'sonnet-4-6', turns: 15, cost: 0.21 },
  { id: 's8', repoId: 'r1', task: 'Move OTLP to daemon',            duration: '38m',    ftr: 1.00, when: '2d ago',  model: 'sonnet-4-6', turns: 10, cost: 0.11 },
  { id: 's9', repoId: 'r4', task: 'Add rate limiting',              duration: '22m',    ftr: 0.88, when: '3d ago',  model: 'sonnet-4-6', turns: 6,  cost: 0.07 },
  { id: 's10',repoId: 'r2', task: 'Component library audit',        duration: '1h 10m', ftr: 0.75, when: '3d ago',  model: 'opus-4-6',   turns: 20, cost: 0.41 },
];
