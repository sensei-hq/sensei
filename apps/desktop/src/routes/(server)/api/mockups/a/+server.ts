import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

// Mockup A: three-pane workspace view data
export const GET: RequestHandler = () => {
  return json({
    projects: [
      { id: 'p1', name: 'sensei', kind: 'repo', maturity: 3, phase: 'Implementation', lastSeen: '2h ago', unread: 2 },
      { id: 'p2', name: 'rokkit', kind: 'repo', maturity: 4, phase: 'Maintenance', lastSeen: 'Yesterday', unread: 0 },
      { id: 'p3', name: 'api-gateway', kind: 'repo', maturity: 2, phase: 'Design', lastSeen: '1w ago', unread: 1 },
      { id: 'p4', name: 'sensei-mobile', kind: 'idea', maturity: 1, phase: 'Ideation', lastSeen: '5d ago', unread: 0 },
      { id: 'p5', name: 'dbd', kind: 'repo', maturity: 3, phase: 'Implementation', lastSeen: '4d ago', unread: 0 },
    ],
    activeProject: {
      id: 'p1',
      name: 'sensei',
      path: '~/Developer/sensei',
      description: 'Claude Code lifecycle guide — local-first desktop app',
      maturity: 3,
      language: 'TypeScript',
      phases: [
        { name: 'Requirements', done: true,  cardCount: 8 },
        { name: 'Analysis',     done: true,  cardCount: 5 },
        { name: 'Design',       done: true,  cardCount: 12 },
        { name: 'Implementation', done: false, cardCount: 7, active: true },
        { name: 'Review',       done: false, cardCount: 0 },
      ],
      cards: [
        {
          id: 'c1', kind: 'decision', tag: 'DECISION', status: 'done',
          title: 'Use SQLite as default — Postgres optional via DatabaseAdapter',
          body: 'FTS5 replaces LIKE-based BM25. WAL handles concurrent reads from desktop. sqlite-vec optional with graceful fallback.',
          linkedSymbols: 2,
        },
        {
          id: 'c2', kind: 'requirement', tag: 'REQ', status: 'done',
          title: 'Repo discovery scans ~/Developer and ~/Work on first launch',
          body: 'Walk directories two levels deep. Show project selector modal with maturity pre-estimated from git history.',
          linkedSymbols: 0,
        },
        {
          id: 'c3', kind: 'task', tag: 'TASK', status: 'active',
          title: 'Build CoordinatorAdapter interface — Claude Code default, opencode/Copilot later',
          body: 'Five capabilities: MCP registration, event capture, skill delivery, session start, project context file.',
          linkedSymbols: 5,
        },
        {
          id: 'c4', kind: 'note', tag: 'NOTE', status: 'open',
          title: 'Graph narrative report surfaces god nodes + rationale nodes together',
          body: 'Top-N highest-degree symbols per Leiden community. Include rationale (NOTE/WHY comments) as first-class context.',
          linkedSymbols: 3,
        },
        {
          id: 'c5', kind: 'question', tag: '?', status: 'open',
          title: 'Should hyperedge co_change be computed on first-index or only on explicit request?',
          body: 'Git history walk is expensive. Defer to explicit /find-orphans or /analyze-repo command.',
          linkedSymbols: 0,
        },
      ],
    },
    activeCard: null,
    sessions: [
      { id: 's1', task: 'Build SQLite schema for global + index databases', status: 'completed', result: 'First try', when: '2h ago', coordinator: 'claude-code' },
      { id: 's2', task: 'Scaffold Tauri desktop app with SvelteKit frontend', status: 'in-progress', result: null, when: 'Now', coordinator: 'claude-code' },
      { id: 's3', task: 'Implement CoordinatorAdapter interface and registry', status: 'open', result: null, when: 'Queued', coordinator: null },
    ],
  });
};
