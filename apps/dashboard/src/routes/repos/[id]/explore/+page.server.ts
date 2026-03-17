import type { PageServerLoad } from './$types';

export const load: PageServerLoad = ({ params }) => ({
  repo: { id: params.id, name: 'sensei' },
  graph: {
    nodes: [
      { id: 'n1', name: 'createSenseiMcpServer', kind: 'function', file: 'packages/server/src/mcp-server.ts', complexity: 8, deps: ['n2', 'n3', 'n4'] },
      { id: 'n2', name: 'getSessionContext', kind: 'function', file: 'packages/server/src/tools/get-session-context.ts', complexity: 5, deps: ['n5'] },
      { id: 'n3', name: 'contextPack', kind: 'function', file: 'packages/server/src/tools/context-pack.ts', complexity: 12, deps: ['n5', 'n6'] },
      { id: 'n4', name: 'search', kind: 'function', file: 'packages/server/src/tools/search.ts', complexity: 4, deps: ['n5'] },
      { id: 'n5', name: 'makeSenseiClient', kind: 'function', file: 'packages/shared/src/supabase-client.ts', complexity: 3, deps: [] },
      { id: 'n6', name: 'OllamaBackend', kind: 'class', file: 'packages/server/src/model/ollama-backend.ts', complexity: 6, deps: [] },
      { id: 'n7', name: 'startDaemon', kind: 'function', file: 'packages/collector/src/daemon.ts', complexity: 9, deps: ['n5'] },
      { id: 'n8', name: 'parseOtlpBody', kind: 'function', file: 'packages/collector/src/daemon.ts', complexity: 5, deps: [] },
    ]
  },
  complexity: [
    { file: 'packages/server/src/mcp-server.ts', functions: 14, avgComplexity: 6.2, maxComplexity: 18, lines: 341, riskLevel: 'high' },
    { file: 'packages/server/src/tools/context-pack.ts', functions: 8, avgComplexity: 8.1, maxComplexity: 22, lines: 287, riskLevel: 'high' },
    { file: 'packages/collector/src/daemon.ts', functions: 6, avgComplexity: 5.3, maxComplexity: 12, lines: 195, riskLevel: 'medium' },
    { file: 'packages/engine/src/agent/claude-adapter.ts', functions: 11, avgComplexity: 4.8, maxComplexity: 9, lines: 234, riskLevel: 'medium' },
    { file: 'packages/shared/src/supabase-client.ts', functions: 3, avgComplexity: 2.1, maxComplexity: 4, lines: 45, riskLevel: 'low' },
    { file: 'packages/cli/src/commands/setup.ts', functions: 5, avgComplexity: 3.8, maxComplexity: 8, lines: 142, riskLevel: 'low' },
    { file: 'packages/tools/src/tools/search.ts', functions: 4, avgComplexity: 3.2, maxComplexity: 6, lines: 98, riskLevel: 'low' },
  ]
});
