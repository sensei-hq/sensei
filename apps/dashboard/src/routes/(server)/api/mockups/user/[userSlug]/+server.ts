import { json } from '@sveltejs/kit';

export function GET() {
  return json({
    userName: 'Bob Kim',
    orgName: 'Acme Corp',
    email: 'bob@acme.com',
    ftr: 91,
    stats: [
      { label: 'Sessions (30d)', value: '412', delta: '↑ 38 vs prev',          up: true  },
      { label: 'My Cost (30d)',  value: '$58',  delta: '↑ $8 (more sessions)', up: false },
      { label: 'Cache Hit Rate', value: '44%', delta: '↑ 6pp vs prev',         up: true  },
    ],
    repos: [
      { icon: '📦', name: 'api-service',    lang: 'TypeScript', symbols: 4201, indexed: '2h ago', ftr: 89, cost: '$0.11' },
      { icon: '🖥',  name: 'web-dashboard', lang: 'SvelteKit',  symbols: 2840, indexed: '1d ago', ftr: 74, cost: '$0.19' },
    ],
    sessions: [
      { task: 'Add rate limiting to auth endpoints', repo: 'api-service',   status: 'completed', result: 'First try',  cost: '$0.12', when: '2h ago' },
      { task: 'Fix pagination bug in /users list',   repo: 'api-service',   status: 'completed', result: 'First try',  cost: '$0.08', when: '5h ago' },
      { task: 'Refactor dashboard chart components', repo: 'web-dashboard', status: 'partial',   result: '2 attempts', cost: '$0.34', when: '1d ago' },
    ],
    setupSteps: ['Install CLI', 'Init repo', 'MCP setup', 'Link account', 'Plugin'],
    setupDoneUpTo: 3,
  });
}
