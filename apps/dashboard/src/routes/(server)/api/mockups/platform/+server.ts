import { json } from '@sveltejs/kit';

export function GET() {
  return json({
    stats: [
      { label: 'Active Tenants',   value: '24',     delta: '↑ 3 this month',   up: true  },
      { label: 'Sessions (30d)',   value: '18,432', delta: '↑ 12% vs prev',    up: true  },
      { label: 'Platform Avg FTR', value: '71%',    delta: '↑ 4pp this month', up: true  },
      { label: 'Avg Task Cost',    value: '$0.18',  delta: '↓ $0.02 (cache↑)', up: false },
    ],
    ftrBuckets: [
      { label: '<40%',  height: 20, colorCls: 'bg-error-z5'   },
      { label: '40–60', height: 45, colorCls: 'bg-warning-z5' },
      { label: '60–80', height: 90, colorCls: 'bg-primary-z5' },
      { label: '80–90', height: 55, colorCls: 'bg-success-z4' },
      { label: '>90%',  height: 25, colorCls: 'bg-success-z6' },
    ],
    mcpTools: [
      { name: 'context_pack',        pct: 38 },
      { name: 'get_session_context', pct: 24 },
      { name: 'checkpoint',          pct: 15 },
      { name: 'search',              pct: 11 },
      { name: 'record_memory',       pct:  7 },
    ],
    tenants: [
      { icon: '🏢', name: 'Acme Corp',    desc: 'Team · 12 users · 8 repos',        ftr: 84, cost: '$0.14', sessions: 2341, badge: 'Healthy',   badgeCls: 'bg-success-z1 text-success-z7 border-success-z3' },
      { icon: '🚀', name: 'DevStudio',    desc: 'Starter · 3 users · 2 repos',      ftr: 61, cost: '$0.31', sessions:  214, badge: 'Improving',  badgeCls: 'bg-warning-z1 text-warning-z7 border-warning-z3' },
      { icon: '🏦', name: 'FinTech Labs', desc: 'Enterprise · 45 users · 23 repos', ftr: 91, cost: '$0.09', sessions: 9812, badge: 'Top 10%',    badgeCls: 'bg-success-z1 text-success-z7 border-success-z3' },
    ],
  });
}
