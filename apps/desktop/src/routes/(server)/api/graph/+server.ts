export const prerender = true;
import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = () => {
  return json({
    summary: {
      totalSymbols: 10271,
      totalEdges: 28450,
      communities: 12,
      godNodeCount: 4,
      rationaleCount: 47,
    },
    projects: [
      { id: 'p1', name: 'sensei',      symbolCount: 4821, communityCount: 5, color: 'bg-primary-z5'   },
      { id: 'p2', name: 'rokkit',      symbolCount: 2340, communityCount: 3, color: 'bg-secondary-z5' },
      { id: 'p3', name: 'kavach',      symbolCount: 1890, communityCount: 2, color: 'bg-success-z5'   },
      { id: 'p4', name: 'dbd',         symbolCount: 1240, communityCount: 2, color: 'bg-warning-z5'   },
    ],
    communities: [
      { id: 0,  project: 'sensei',  label: 'Agent Layer',       symbolCount: 142, godNodes: ['CoordinatorAdapter', 'AgentAdapter'],      color: 'bg-primary-z5'   },
      { id: 1,  project: 'sensei',  label: 'Graph Engine',      symbolCount: 98,  godNodes: ['rankBfs', 'rankBm25'],                     color: 'bg-secondary-z5' },
      { id: 2,  project: 'sensei',  label: 'MCP Protocol',      symbolCount: 87,  godNodes: ['get_session_context', 'checkpoint'],       color: 'bg-success-z5'   },
      { id: 3,  project: 'sensei',  label: 'Data Layer',        symbolCount: 76,  godNodes: ['initGlobalDb', 'initIndexDb'],             color: 'bg-warning-z5'   },
      { id: 4,  project: 'sensei',  label: 'CLI Commands',      symbolCount: 54,  godNodes: ['setupAgent', 'installSkills'],             color: 'bg-info-z5'      },
      { id: 5,  project: 'rokkit',  label: 'Theme System',      symbolCount: 87,  godNodes: ['presetRokkit', 'ThemeSwitcherToggle'],     color: 'bg-secondary-z5' },
      { id: 6,  project: 'rokkit',  label: 'Component Core',    symbolCount: 76,  godNodes: ['Button', 'Input'],                        color: 'bg-primary-z4'   },
      { id: 7,  project: 'kavach',  label: 'Auth Core',         symbolCount: 120, godNodes: ['kavach', 'validateSession'],              color: 'bg-success-z5'   },
      { id: 8,  project: 'kavach',  label: 'Route Guards',      symbolCount: 70,  godNodes: ['protect', 'redirect'],                   color: 'bg-success-z4'   },
    ],
    godNodes: [
      { name: 'CoordinatorAdapter',   project: 'sensei',  degree: 18, community: 'Agent Layer',    file: 'packages/engine/src/agent/agent-adapter.ts'    },
      { name: 'rankBfs',              project: 'sensei',  degree: 14, community: 'Graph Engine',   file: 'packages/engine/src/db/queries.ts'              },
      { name: 'get_session_context',  project: 'sensei',  degree: 12, community: 'MCP Protocol',  file: 'packages/server/src/tools/session.ts'           },
      { name: 'initGlobalDb',         project: 'sensei',  degree: 10, community: 'Data Layer',    file: 'packages/shared/src/db/init.ts'                 },
      { name: 'kavach',               project: 'kavach',  degree: 22, community: 'Auth Core',     file: 'packages/core/src/index.ts'                     },
      { name: 'presetRokkit',         project: 'rokkit',  degree: 19, community: 'Theme System',  file: 'packages/unocss/src/preset.ts'                  },
    ],
    rationale: [
      { tag: 'WHY',      project: 'sensei',  file: 'packages/engine/src/db/queries.ts:14',          text: 'FTS5 is a real inverted index — faster than LIKE for BM25' },
      { tag: 'DECISION', project: 'sensei',  file: 'packages/engine/src/agent/claude-adapter.ts:8', text: 'Hardcoded to ~/.claude/skills/ pending CoordinatorAdapter refactor' },
      { tag: 'NOTE',     project: 'sensei',  file: 'packages/collector/src/install.ts:31',           text: 'launchd plist is macOS-only — need cross-platform daemon lifecycle' },
      { tag: 'WHY',      project: 'kavach',  file: 'packages/core/src/middleware.ts:22',             text: 'Session token stored in httpOnly cookie to prevent XSS access' },
      { tag: 'HACK',     project: 'rokkit',  file: 'packages/unocss/src/preset.ts:89',              text: 'Manual safelist needed for dynamic icon classes — UnoCSS cannot detect at-runtime strings' },
    ],
  });
};
