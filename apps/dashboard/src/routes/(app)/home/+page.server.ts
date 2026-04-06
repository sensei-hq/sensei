import type { PageServerLoad } from './$types';

export const load: PageServerLoad = ({ locals }) => {
  const user = locals.session?.user;
  return {
    userName: user?.full_name ?? user?.email ?? 'You',
    orgName: 'Seneca Global',
    email: user?.email ?? '',
    ftr: 91,
    stats: [
      { label: 'Sessions (30d)', value: '412', delta: '↑ 38 vs prev',          up: true  },
      { label: 'My Cost (30d)',  value: '$58',  delta: '↑ $8 (more sessions)', up: false },
      { label: 'Cache Hit Rate', value: '44%', delta: '↑ 6pp vs prev',         up: true  },
    ],
    repos: [
      { icon: '📦', name: 'sensei',        lang: 'TypeScript', symbols: 5102, indexed: '1h ago', ftr: 91, cost: '$0.09' },
      { icon: '🖥',  name: 'sensei-dash',  lang: 'SvelteKit',  symbols: 2840, indexed: '3h ago', ftr: 84, cost: '$0.14' },
    ],
    sessions: [
      { task: 'Add role-based route protection',     repo: 'sensei',      status: 'completed', result: 'First try',  cost: '$0.09', when: '1h ago'  },
      { task: 'Fix logout redirect in sidebar',      repo: 'sensei-dash', status: 'completed', result: 'First try',  cost: '$0.07', when: '4h ago'  },
      { task: 'Migrate kavach deps to bun link',     repo: 'sensei',      status: 'completed', result: 'First try',  cost: '$0.11', when: '1d ago'  },
    ],
    setupSteps: ['Install CLI', 'Init repo', 'MCP setup', 'Link account', 'Plugin'],
    setupDoneUpTo: 3,
  };
};
