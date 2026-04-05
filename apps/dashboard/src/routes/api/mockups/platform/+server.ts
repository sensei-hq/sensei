import { json } from '@sveltejs/kit';

export function GET() {
  return json({
    dev: {
      stats: [
        { label: 'Sessions (30d)', value: '412', delta: '↑ 38 vs prev',          up: true  },
        { label: 'My Cost (30d)',  value: '$58',  delta: '↑ $8 (more sessions)', up: false },
        { label: 'Cache Hit Rate', value: '44%', delta: '↑ 6pp vs prev',         up: true  },
      ],
      ftr: 91,
      repos: [
        { icon: '📦', name: 'api-service',    lang: 'TypeScript', symbols: 4201, indexed: '2h ago', ftr: 89, cost: '$0.11' },
        { icon: '🖥',  name: 'web-dashboard', lang: 'SvelteKit',  symbols: 2840, indexed: '1d ago', ftr: 74, cost: '$0.19' },
      ],
      sessions: [
        { task: 'Add rate limiting to auth endpoints',   repo: 'api-service',   status: 'completed', result: 'First try',  cost: '$0.12', when: '2h ago' },
        { task: 'Fix pagination bug in /users list',     repo: 'api-service',   status: 'completed', result: 'First try',  cost: '$0.08', when: '5h ago' },
        { task: 'Refactor dashboard chart components',   repo: 'web-dashboard', status: 'partial',   result: '2 attempts', cost: '$0.34', when: '1d ago' },
      ],
      setupSteps: ['Install CLI', 'Init repo', 'MCP setup', 'Link account', 'Plugin'],
      setupDoneUpTo: 3,
    },
    org: {
      stats: [
        { label: 'Team FTR (30d)',   value: '84%',  delta: '↑ 7pp vs last month', up: true  },
        { label: 'Active Members',   value: '9/12', delta: '3 pending invite',     up: null  },
        { label: 'Total Cost (30d)', value: '$327', delta: '↑ $42 vs last month',  up: false },
        { label: 'Repos Indexed',    value: '8',    delta: '↑ 2 new this month',   up: true  },
      ],
      contributors: [
        { initials: 'BK', name: 'Bob Kim',     ftr: 91, sessions: 412, cost: '$58' },
        { initials: 'CS', name: 'Carol Singh',  ftr: 87, sessions: 388, cost: '$62' },
        { initials: 'AC', name: 'Alice Chen',   ftr: 79, sessions: 201, cost: '$31' },
      ],
      repos: [
        { icon: '📦', name: 'api-service',    desc: '4,201 symbols · indexed 2h ago', ftr: 89, badge: 'FTR 89%',     badgeCls: 'bg-success-z1 text-success-z7 border-success-z3' },
        { icon: '🖥',  name: 'web-dashboard', desc: '2,840 symbols · indexed 1d ago',  ftr: 71, badge: 'FTR 71%',     badgeCls: 'bg-warning-z1 text-warning-z7 border-warning-z3' },
        { icon: '🔧', name: 'data-pipeline',  desc: '1,120 symbols · indexed 5d ago',  ftr:  0, badge: 'Stale index', badgeCls: 'bg-error-z1 text-error-z7 border-error-z3'      },
      ],
      pending: [
        { email: 'dave@acme.com', invited: '2d ago', status: 'Invite sent',              statusCls: 'bg-warning-z1 text-warning-z7 border-warning-z3' },
        { email: 'eve@acme.com',  invited: '1d ago', status: 'Signed up · setup pending', statusCls: 'bg-primary-z1 text-primary-z7 border-primary-z3' },
      ],
    },
    platform: {
      stats: [
        { label: 'Active Tenants',   value: '24',     delta: '↑ 3 this month',   up: true  },
        { label: 'Sessions (30d)',   value: '18,432', delta: '↑ 12% vs prev',    up: true  },
        { label: 'Platform Avg FTR', value: '71%',    delta: '↑ 4pp this month', up: true  },
        { label: 'Avg Task Cost',    value: '$0.18',  delta: '↓ $0.02 (cache↑)', up: false },
      ],
      ftrBuckets: [
        { label: '<40%',  height: 20, colorCls: 'bg-error-z5'    },
        { label: '40–60', height: 45, colorCls: 'bg-warning-z5'  },
        { label: '60–80', height: 90, colorCls: 'bg-primary-z5'  },
        { label: '80–90', height: 55, colorCls: 'bg-success-z4'  },
        { label: '>90%',  height: 25, colorCls: 'bg-success-z6'  },
      ],
      mcpTools: [
        { name: 'context_pack',        pct: 38 },
        { name: 'get_session_context', pct: 24 },
        { name: 'checkpoint',          pct: 15 },
        { name: 'search',              pct: 11 },
        { name: 'record_memory',       pct:  7 },
      ],
      tenants: [
        { icon: '🏢', name: 'Acme Corp',    desc: 'Team · 12 users · 8 repos',        ftr: 84, cost: '$0.14', sessions: 2341, badge: 'Healthy',  badgeCls: 'bg-success-z1 text-success-z7 border-success-z3' },
        { icon: '🚀', name: 'DevStudio',    desc: 'Starter · 3 users · 2 repos',      ftr: 61, cost: '$0.31', sessions:  214, badge: 'Improving', badgeCls: 'bg-warning-z1 text-warning-z7 border-warning-z3' },
        { icon: '🏦', name: 'FinTech Labs', desc: 'Enterprise · 45 users · 23 repos', ftr: 91, cost: '$0.09', sessions: 9812, badge: 'Top 10%',   badgeCls: 'bg-success-z1 text-success-z7 border-success-z3' },
      ],
    },
  });
}
