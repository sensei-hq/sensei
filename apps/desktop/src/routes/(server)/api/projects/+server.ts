import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

// Seed data — real impl reads from ~/.sensei/sensei.db
// and scans ~/Developer + ~/Work
export const GET: RequestHandler = () => {
  return json([
    {
      id: 'proj-001', kind: 'repo',
      name: 'sensei', path: '~/Developer/sensei',
      description: 'Claude Code lifecycle guide — local-first desktop app',
      language: 'TypeScript', maturity: 3,
      activePhase: 'Implementation', lastActivity: '2 hours ago',
      sessionCount: 24, cardCount: 12, symbolCount: 4821, ftrScore: 0.72,
      phases: [
        { name: 'Requirements',   done: true,  active: false, cardCount: 8  },
        { name: 'Analysis',       done: true,  active: false, cardCount: 5  },
        { name: 'Design',         done: true,  active: false, cardCount: 12 },
        { name: 'Implementation', done: false, active: true,  cardCount: 7  },
        { name: 'Review',         done: false, active: false, cardCount: 0  },
      ],
      cards: [
        { id: 'c1', kind: 'decision',    tag: 'DECISION', status: 'done',   title: 'Use SQLite as default — Postgres optional via DatabaseAdapter',                    body: 'FTS5 replaces LIKE-based BM25. WAL handles concurrent reads. sqlite-vec optional with graceful FTS5 fallback.',          linkedSymbols: 2, updatedAt: '2h ago'  },
        { id: 'c2', kind: 'requirement', tag: 'REQ',      status: 'done',   title: 'Repo discovery scans ~/Developer and ~/Work on first launch',                       body: 'Walk directories two levels deep. Show project selector modal. Pre-estimate maturity from git history.',                 linkedSymbols: 0, updatedAt: '1d ago'  },
        { id: 'c3', kind: 'task',        tag: 'TASK',     status: 'active', title: 'CoordinatorAdapter interface — Claude Code default, opencode/Copilot later',        body: 'Five capabilities: MCP registration, event capture, skill delivery, session start, project context file.',               linkedSymbols: 5, updatedAt: 'Now'     },
        { id: 'c4', kind: 'note',        tag: 'NOTE',     status: 'open',   title: 'Graph narrative report surfaces god nodes + rationale nodes together',              body: 'Top-N highest-degree symbols per Leiden community. Include NOTE/WHY comments as first-class context.',                   linkedSymbols: 3, updatedAt: '3d ago'  },
        { id: 'c5', kind: 'question',    tag: '?',        status: 'open',   title: 'co_change hyperedges — compute eagerly on index or only on /analyze-repo?',         body: 'Git history walk is expensive. Defer to explicit command to avoid blocking incremental index runs.',                       linkedSymbols: 0, updatedAt: '5d ago'  },
        { id: 'c6', kind: 'finding',     tag: 'FINDING',  status: 'open',   title: 'setupAgent() hard-rejects non-Claude agents — must abstract (gap #4)',             body: 'Line 42 in packages/cli/src/commands/setup.ts: if (agent !== "claude") process.exit(1). Blocks coordinator adapter.',   linkedSymbols: 1, updatedAt: '1w ago'  },
        { id: 'c7', kind: 'decision',    tag: 'DECISION', status: 'done',   title: 'Two databases: ~/.sensei/sensei.db (global) + <repo>/.sensei/index.db (per-repo)', body: 'Remove repo_id from index tables — the per-repo db IS the scope. Cross-repo queries go via sensei.db projects table.',   linkedSymbols: 4, updatedAt: '2d ago'  },
      ],
      godNodes: [
        { name: 'CoordinatorAdapter', degree: 18, community: 'Agent Layer'  },
        { name: 'rankBfs',            degree: 14, community: 'Graph Engine' },
        { name: 'get_session_context', degree: 12, community: 'MCP'        },
        { name: 'initGlobalDb',       degree: 10, community: 'Data Layer'  },
      ],
      communities: [
        { id: 0, label: 'Agent Layer',  symbolCount: 142, color: 'bg-primary-z5'   },
        { id: 1, label: 'Graph Engine', symbolCount: 98,  color: 'bg-secondary-z5' },
        { id: 2, label: 'MCP Protocol', symbolCount: 87,  color: 'bg-success-z5'   },
        { id: 3, label: 'Data Layer',   symbolCount: 76,  color: 'bg-warning-z5'   },
        { id: 4, label: 'CLI Commands', symbolCount: 54,  color: 'bg-info-z5'      },
      ],
      rationale: [
        { tag: 'WHY',      file: 'packages/engine/src/db/queries.ts:14',         text: 'FTS5 is a real inverted index — substantially faster than LIKE for BM25 approximation' },
        { tag: 'DECISION', file: 'packages/engine/src/agent/claude-adapter.ts:8', text: 'Hardcoded to ~/.claude/skills/ pending CoordinatorAdapter refactor' },
        { tag: 'NOTE',     file: 'packages/collector/src/install.ts:31',         text: 'launchd plist is macOS-only — need cross-platform daemon lifecycle' },
      ],
    },
    {
      id: 'proj-002', kind: 'repo',
      name: 'rokkit', path: '~/Developer/rokkit',
      description: 'Design system and component library for Sensei ecosystem',
      language: 'TypeScript', maturity: 4,
      activePhase: 'Maintenance', lastActivity: 'Yesterday',
      sessionCount: 18, cardCount: 8, symbolCount: 2340, ftrScore: 0.85,
      phases: [
        { name: 'Requirements',   done: true, active: false, cardCount: 4 },
        { name: 'Analysis',       done: true, active: false, cardCount: 2 },
        { name: 'Design',         done: true, active: false, cardCount: 6 },
        { name: 'Implementation', done: true, active: false, cardCount: 8 },
        { name: 'Review',         done: true, active: true,  cardCount: 3 },
      ],
      cards: [
        { id: 'r1', kind: 'note',     tag: 'NOTE',     status: 'open',   title: 'presetRokkit already includes extractorSvelte and transformerDirectives', body: 'No need to add them separately in uno.config.js.',        linkedSymbols: 1, updatedAt: '3d ago' },
        { id: 'r2', kind: 'finding',  tag: 'FINDING',  status: 'open',   title: 'SearchFilter.svelte type error: Object[] vs FilterObject[]',              body: 'Known bug in @rokkit/ui. Filed issue. Do not workaround.', linkedSymbols: 0, updatedAt: '1w ago' },
      ],
      godNodes: [],
      communities: [],
      rationale: [],
    },
    {
      id: 'proj-003', kind: 'repo',
      name: 'kavach', path: '~/Developer/kavach',
      description: 'Authentication middleware for SvelteKit applications',
      language: 'TypeScript', maturity: 4,
      activePhase: 'Maintenance', lastActivity: '3 days ago',
      sessionCount: 31, cardCount: 5, symbolCount: 1890, ftrScore: 0.91,
      phases: [
        { name: 'Requirements',   done: true, active: false, cardCount: 3 },
        { name: 'Analysis',       done: true, active: false, cardCount: 2 },
        { name: 'Design',         done: true, active: false, cardCount: 4 },
        { name: 'Implementation', done: true, active: false, cardCount: 9 },
        { name: 'Review',         done: true, active: true,  cardCount: 2 },
      ],
      cards: [
        { id: 'k1', kind: 'note', tag: 'NOTE', status: 'open', title: 'ESM cache issue requires bun run dev restart after adding new rules', body: 'Known kavach behaviour — restart to activate new route rules.', linkedSymbols: 0, updatedAt: '5d ago' },
      ],
      godNodes: [],
      communities: [],
      rationale: [],
    },
    {
      id: 'proj-004', kind: 'repo',
      name: 'api-gateway', path: '~/Work/api-gateway',
      description: 'Internal API gateway and rate limiting service',
      language: 'Go', maturity: 2,
      activePhase: 'Design', lastActivity: '1 week ago',
      sessionCount: 7, cardCount: 14, symbolCount: 980, ftrScore: 0.68,
      phases: [
        { name: 'Requirements',   done: true,  active: false, cardCount: 6 },
        { name: 'Analysis',       done: true,  active: false, cardCount: 4 },
        { name: 'Design',         done: false, active: true,  cardCount: 4 },
        { name: 'Implementation', done: false, active: false, cardCount: 0 },
        { name: 'Review',         done: false, active: false, cardCount: 0 },
      ],
      cards: [
        { id: 'a1', kind: 'requirement', tag: 'REQ',  status: 'done',   title: 'Rate limit per API key with sliding window algorithm',  body: 'Redis-backed sliding window. 1000 req/min default, configurable per key.', linkedSymbols: 0, updatedAt: '1w ago' },
        { id: 'a2', kind: 'question',    tag: '?',    status: 'open',   title: 'Should gateway handle auth or delegate to upstream?',   body: 'Current leaning: validate JWT at gateway, pass claims downstream.',        linkedSymbols: 0, updatedAt: '4d ago' },
        { id: 'a3', kind: 'task',        tag: 'TASK', status: 'active', title: 'Design routing config format — YAML or code-based?',    body: 'YAML for ops-friendly, code for flexibility. Hybrid approach?',            linkedSymbols: 0, updatedAt: '2d ago' },
      ],
      godNodes: [],
      communities: [],
      rationale: [],
    },
    {
      id: 'proj-005', kind: 'repo',
      name: 'dbd', path: '~/Developer/dbd',
      description: 'Database DDL management and migration tool',
      language: 'TypeScript', maturity: 3,
      activePhase: 'Implementation', lastActivity: '4 days ago',
      sessionCount: 11, cardCount: 6, symbolCount: 1240, ftrScore: 0.78,
      phases: [
        { name: 'Requirements',   done: true,  active: false, cardCount: 3 },
        { name: 'Analysis',       done: true,  active: false, cardCount: 2 },
        { name: 'Design',         done: true,  active: false, cardCount: 4 },
        { name: 'Implementation', done: false, active: true,  cardCount: 3 },
        { name: 'Review',         done: false, active: false, cardCount: 0 },
      ],
      cards: [
        { id: 'd1', kind: 'note', tag: 'NOTE', status: 'open', title: 'dbd manages DDL source of truth in database/ddl/sqlite/', body: 'Migrations generated from diffs between DDL versions. Never edit supabase/migrations directly.', linkedSymbols: 0, updatedAt: '4d ago' },
      ],
      godNodes: [],
      communities: [],
      rationale: [],
    },
  ]);
};
