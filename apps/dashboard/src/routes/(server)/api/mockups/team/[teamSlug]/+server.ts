import { json } from '@sveltejs/kit';

export function GET() {
  return json({
    teamName: 'Acme Corp',
    stats: [
      { label: 'Team FTR (30d)',   value: '84%',  delta: '↑ 7pp vs last month', up: true  },
      { label: 'Active Members',   value: '9/12', delta: '3 pending invite',     up: null  },
      { label: 'Total Cost (30d)', value: '$327', delta: '↑ $42 vs last month',  up: false },
      { label: 'Repos Indexed',    value: '8',    delta: '↑ 2 new this month',   up: true  },
    ],
    contributors: [
      { initials: 'BK', name: 'Bob Kim',    ftr: 91, sessions: 412, cost: '$58' },
      { initials: 'CS', name: 'Carol Singh', ftr: 87, sessions: 388, cost: '$62' },
      { initials: 'AC', name: 'Alice Chen',  ftr: 79, sessions: 201, cost: '$31' },
    ],
    repos: [
      { icon: '📦', name: 'api-service',   desc: '4,201 symbols · indexed 2h ago', badge: 'FTR 89%',     badgeCls: 'bg-success-z1 text-success-z7 border-success-z3' },
      { icon: '🖥',  name: 'web-dashboard', desc: '2,840 symbols · indexed 1d ago',  badge: 'FTR 71%',     badgeCls: 'bg-warning-z1 text-warning-z7 border-warning-z3' },
      { icon: '🔧', name: 'data-pipeline',  desc: '1,120 symbols · indexed 5d ago',  badge: 'Stale index', badgeCls: 'bg-error-z1 text-error-z7 border-error-z3'      },
    ],
    pending: [
      { email: 'dave@acme.com', invited: '2d ago', status: 'Invite sent',               statusCls: 'bg-warning-z1 text-warning-z7 border-warning-z3' },
      { email: 'eve@acme.com',  invited: '1d ago', status: 'Signed up · setup pending', statusCls: 'bg-primary-z1 text-primary-z7 border-primary-z3' },
    ],
  });
}
