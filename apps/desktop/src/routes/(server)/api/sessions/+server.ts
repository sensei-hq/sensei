export const prerender = true;
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = () => {
  return json({
    stats: {
      totalSessions: 91,
      completedSessions: 78,
      totalTurns: 1240,
      avgFtr: 0.78,
      totalCost: 24.30,
      totalTokensIn: 3_820_000,
      totalTokensOut: 580_000,
    },
    sessions: [
      { id: 's01', project: 'sensei',      task: 'Scaffold Tauri desktop app with SvelteKit frontend',       status: 'in-progress', turns: 24, cost: 0.92, tokens: { in: 210_000, out: 28_000 }, ftr: null, when: 'Now',       date: '2026-04-08T14:00:00Z' },
      { id: 's02', project: 'sensei',      task: 'Build SQLite schema for global + index databases',         status: 'completed',   turns: 31, cost: 1.10, tokens: { in: 290_000, out: 41_000 }, ftr: 1.00, when: '2h ago',    date: '2026-04-08T12:00:00Z' },
      { id: 's03', project: 'sensei',      task: 'Create CoordinatorAdapter interface and registry',         status: 'completed',   turns: 18, cost: 0.68, tokens: { in: 180_000, out: 22_000 }, ftr: 1.00, when: '5h ago',    date: '2026-04-08T09:00:00Z' },
      { id: 's04', project: 'sensei',      task: 'Write roadmap docs: paradigm shift + local architecture', status: 'completed',   turns: 22, cost: 0.84, tokens: { in: 220_000, out: 34_000 }, ftr: 1.00, when: 'Yesterday', date: '2026-04-07T16:00:00Z' },
      { id: 's05', project: 'rokkit',      task: 'Fix ThemeSwitcherToggle type error in MessagesStore',     status: 'completed',   turns: 14, cost: 0.52, tokens: { in: 140_000, out: 18_000 }, ftr: 0.60, when: 'Yesterday', date: '2026-04-07T11:00:00Z' },
      { id: 's06', project: 'api-gateway', task: 'Design routing config — YAML vs code-based decision',     status: 'completed',   turns: 11, cost: 0.41, tokens: { in: 110_000, out: 14_000 }, ftr: 1.00, when: '2d ago',    date: '2026-04-06T14:00:00Z' },
      { id: 's07', project: 'sensei',      task: 'Restructure database DDL — policies, RLS, views',         status: 'completed',   turns: 27, cost: 1.02, tokens: { in: 270_000, out: 38_000 }, ftr: 0.50, when: '3d ago',    date: '2026-04-05T10:00:00Z' },
      { id: 's08', project: 'kavach',      task: 'Implement org membership with RLS policies',              status: 'completed',   turns: 19, cost: 0.71, tokens: { in: 190_000, out: 25_000 }, ftr: 1.00, when: '4d ago',    date: '2026-04-04T09:00:00Z' },
      { id: 's09', project: 'sensei',      task: 'Wire all dashboard pages to real Supabase data',          status: 'completed',   turns: 35, cost: 1.32, tokens: { in: 350_000, out: 52_000 }, ftr: 0.50, when: '5d ago',    date: '2026-04-03T15:00:00Z' },
      { id: 's10', project: 'dbd',         task: 'Add SQLite DDL support to schema migration tool',         status: 'completed',   turns: 16, cost: 0.60, tokens: { in: 160_000, out: 20_000 }, ftr: 1.00, when: '1w ago',    date: '2026-04-01T10:00:00Z' },
    ],
    toolUsage: [
      { tool: 'Read',  calls: 412, successRate: 0.99, avgDurationMs: 18   },
      { tool: 'Edit',  calls: 298, successRate: 0.97, avgDurationMs: 24   },
      { tool: 'Bash',  calls: 187, successRate: 0.91, avgDurationMs: 1420 },
      { tool: 'Grep',  calls: 143, successRate: 1.00, avgDurationMs: 32   },
      { tool: 'Glob',  calls: 98,  successRate: 1.00, avgDurationMs: 12   },
      { tool: 'Write', calls: 76,  successRate: 0.96, avgDurationMs: 21   },
      { tool: 'Agent', calls: 26,  successRate: 0.88, avgDurationMs: 8400 },
    ],
    benchmarkPairs: [
      { task: 'Add SQLite DDL support',       branch: 'feature/sqlite-ddl',  withoutSensei: 1.84, withSensei: 0.60 },
      { task: 'Implement RLS policies',       branch: 'feature/rls',         withoutSensei: 2.10, withSensei: 0.71 },
      { task: 'CoordinatorAdapter interface', branch: 'feature/coordinator', withoutSensei: 1.95, withSensei: 0.68 },
    ],
  });
};
