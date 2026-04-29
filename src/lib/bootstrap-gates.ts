/**
 * Bootstrap gate definitions — matches the mockup in docs/mockups/lib/bootstrap.jsx.
 * Static data. Status is managed by BootstrapState class.
 */

export type GateStatus = 'pending' | 'checking' | 'ready' | 'missing' | 'error' | 'starting';

export interface GateDefinition {
  id: string;
  n: string;
  name: string;
  detail: string;
  check: string;
  remedy: 'install' | 'prereq' | 'db' | 'daemon';
  sub?: SubCheckDefinition[];
}

export interface SubCheckDefinition {
  id: string;
  name: string;
  check: string;
}

export const GATES: GateDefinition[] = [
  {
    id: 'homebrew', n: '一', name: 'Homebrew',
    detail: 'package manager',
    check: 'which brew',
    remedy: 'install',
  },
  {
    id: 'postgres', n: '二', name: 'PostgreSQL',
    detail: 'storage · @17',
    check: 'which postgres',
    remedy: 'prereq',
  },
  {
    id: 'ollama', n: '三', name: 'Ollama',
    detail: 'local models for embeddings',
    check: 'which ollama',
    remedy: 'prereq',
  },
  {
    id: 'sensei', n: '四', name: 'Sensei components',
    detail: 'MCP · CLI · daemon',
    check: 'sensei --version',
    remedy: 'prereq',
    sub: [
      { id: 'cli', name: 'sensei-cli', check: 'sensei --version' },
      { id: 'mcp', name: 'MCP bridge', check: 'sensei-mcp --version' },
      { id: 'daemon', name: 'sensei-daemon', check: 'senseid --help' },
    ],
  },
  {
    id: 'database', n: '五', name: 'Database',
    detail: 'sensei schema · pgvector',
    check: 'psql sensei -c "SELECT 1"',
    remedy: 'db',
  },
  {
    id: 'senseid', n: '六', name: 'Daemon',
    detail: 'background observer',
    check: 'curl localhost:7744/health',
    remedy: 'daemon',
  },
];
