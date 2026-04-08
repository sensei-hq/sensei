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
      { id: 's01', project: 'sensei',      task: 'Scaffold Tauri desktop app with SvelteKit frontend',          status: 'in-progress', outcome: null,        turns: 24, cost: 0.92, ftr: null, when: 'Now',       coordinator: 'claude-code' },
      { id: 's02', project: 'sensei',      task: 'Build SQLite schema for global + index databases',            status: 'completed',   outcome: 'completed', turns: 31, cost: 1.10, ftr: 1.00, when: '2h ago',    coordinator: 'claude-code' },
      { id: 's03', project: 'sensei',      task: 'Create CoordinatorAdapter interface and registry',            status: 'completed',   outcome: 'completed', turns: 18, cost: 0.68, ftr: 1.00, when: '5h ago',    coordinator: 'claude-code' },
      { id: 's04', project: 'sensei',      task: 'Write roadmap docs: paradigm shift + local architecture',    status: 'completed',   outcome: 'completed', turns: 22, cost: 0.84, ftr: 1.00, when: 'Yesterday', coordinator: 'claude-code' },
      { id: 's05', project: 'rokkit',      task: 'Fix ThemeSwitcherToggle type error in MessagesStore',        status: 'completed',   outcome: 'partial',   turns: 14, cost: 0.52, ftr: null, when: 'Yesterday', coordinator: 'claude-code' },
      { id: 's06', project: 'api-gateway', task: 'Design routing config format — YAML vs code-based decision', status: 'completed',   outcome: 'completed', turns: 11, cost: 0.41, ftr: 1.00, when: '2d ago',    coordinator: 'claude-code' },
      { id: 's07', project: 'sensei',      task: 'Restructure database DDL — policies folder, RLS, views',     status: 'completed',   outcome: 'completed', turns: 27, cost: 1.02, ftr: 0.50, when: '3d ago',    coordinator: 'claude-code' },
      { id: 's08', project: 'kavach',      task: 'Implement org membership with RLS policies',                  status: 'completed',   outcome: 'completed', turns: 19, cost: 0.71, ftr: 1.00, when: '4d ago',    coordinator: 'claude-code' },
      { id: 's09', project: 'sensei',      task: 'Wire all dashboard pages to real Supabase data',              status: 'completed',   outcome: 'completed', turns: 35, cost: 1.32, ftr: 0.50, when: '5d ago',    coordinator: 'claude-code' },
      { id: 's10', project: 'dbd',         task: 'Add SQLite DDL support to schema migration tool',             status: 'completed',   outcome: 'completed', turns: 16, cost: 0.60, ftr: 1.00, when: '1w ago',    coordinator: 'claude-code' },
    ],
  });
};
