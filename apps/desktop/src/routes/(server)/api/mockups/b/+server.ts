import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

// Mockup B: card workspace (grid) + graph intelligence panel
export const GET: RequestHandler = () => {
  return json({
    project: {
      name: 'sensei',
      path: '~/Developer/sensei',
      maturity: 3,
      phase: 'Implementation',
      ftrScore: 0.72,
      godNodes: [
        { name: 'CoordinatorAdapter', degree: 18, community: 'Agent Layer'     },
        { name: 'rankBfs',            degree: 14, community: 'Graph Engine'    },
        { name: 'get_session_context', degree: 12, community: 'MCP Protocol'   },
        { name: 'initGlobalDb',       degree: 10, community: 'Data Layer'      },
      ],
      communities: [
        { id: 0, label: 'Agent Layer',    symbolCount: 142, color: 'bg-primary-z5'   },
        { id: 1, label: 'Graph Engine',   symbolCount: 98,  color: 'bg-secondary-z5' },
        { id: 2, label: 'MCP Protocol',   symbolCount: 87,  color: 'bg-success-z5'   },
        { id: 3, label: 'Data Layer',     symbolCount: 76,  color: 'bg-warning-z5'   },
        { id: 4, label: 'CLI Commands',   symbolCount: 54,  color: 'bg-info-z5'      },
      ],
    },
    cards: [
      {
        id: 'c1', kind: 'decision', status: 'done', tag: 'DECISION',
        title: 'SQLite as default database',
        summary: 'FTS5 replaces LIKE-based BM25. WAL handles concurrent reads.',
        phase: 'Design', linkedSymbols: 2, updatedAt: '2h ago',
      },
      {
        id: 'c2', kind: 'requirement', status: 'done', tag: 'REQ',
        title: 'Repo discovery from ~/Developer + ~/Work',
        summary: 'Walk directories on first launch. Show project selector modal.',
        phase: 'Requirements', linkedSymbols: 0, updatedAt: '1d ago',
      },
      {
        id: 'c3', kind: 'task', status: 'active', tag: 'TASK',
        title: 'CoordinatorAdapter interface',
        summary: 'Five capabilities: MCP, events, skills, session start, context file.',
        phase: 'Implementation', linkedSymbols: 5, updatedAt: 'Now',
      },
      {
        id: 'c4', kind: 'note', status: 'open', tag: 'NOTE',
        title: 'Graph narrative surfaces god nodes + rationale together',
        summary: 'Top-N highest-degree per Leiden community with WHY annotations.',
        phase: 'Design', linkedSymbols: 3, updatedAt: '3d ago',
      },
      {
        id: 'c5', kind: 'question', status: 'open', tag: '?',
        title: 'co_change hyperedges — compute eagerly or on-demand?',
        summary: 'Git history walk is expensive. Defer to explicit /analyze-repo.',
        phase: 'Implementation', linkedSymbols: 0, updatedAt: '5d ago',
      },
      {
        id: 'c6', kind: 'finding', status: 'open', tag: 'FINDING',
        title: 'setupAgent() hard-rejects non-Claude coordinators (gap)',
        summary: 'Line 42: if (agent !== "claude") process.exit(1). Must abstract.',
        phase: 'Analysis', linkedSymbols: 1, updatedAt: '1w ago',
      },
    ],
    rationale: [
      { tag: 'WHY',      file: 'packages/engine/src/db/queries.ts:14',     text: 'FTS5 is a real inverted index — substantially faster than LIKE for BM25 approximation' },
      { tag: 'DECISION', file: 'packages/engine/src/agent/claude-adapter.ts:8', text: 'Hardcoded to ~/.claude/skills/ pending CoordinatorAdapter refactor' },
      { tag: 'NOTE',     file: 'packages/collector/src/install.ts:31',     text: 'launchd plist is macOS-only — need cross-platform daemon lifecycle' },
    ],
  });
};
