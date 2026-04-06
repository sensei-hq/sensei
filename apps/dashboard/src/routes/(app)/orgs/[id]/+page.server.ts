import type { PageServerLoad } from './$types';

export const load: PageServerLoad = () => ({
  org: { id: 'o1', name: 'Acme Corp', slug: 'acme', plan: 'team', description: 'Main company workspace', createdAt: '2025-01-15T00:00:00Z' },
  teams: [
    { id: 't1', name: 'Platform', description: 'Core infrastructure team', memberCount: 4, repoCount: 3 },
    { id: 't2', name: 'Frontend', description: 'UI and design systems', memberCount: 3, repoCount: 2 },
    { id: 't3', name: 'Backend', description: 'API and data services', memberCount: 5, repoCount: 4 },
  ],
  members: [
    { id: 'u1', name: 'Jerry Thomas', email: 'jerry@acme.com', role: 'admin', teams: ['Platform', 'Frontend'], joinedAt: '2025-01-15T00:00:00Z' },
    { id: 'u2', name: 'Alex Chen', email: 'alex@acme.com', role: 'member', teams: ['Backend'], joinedAt: '2025-02-01T00:00:00Z' },
    { id: 'u3', name: 'Sam Rivera', email: 'sam@acme.com', role: 'member', teams: ['Frontend'], joinedAt: '2025-02-10T00:00:00Z' },
    { id: 'u4', name: 'Priya Patel', email: 'priya@acme.com', role: 'member', teams: ['Backend', 'Platform'], joinedAt: '2025-03-01T00:00:00Z' },
  ],
  repos: [
    { id: 'r1', name: 'sensei', remoteUrl: 'https://github.com/acme/sensei', provider: 'github', lastIndexedAt: '2026-03-17T00:00:00Z' },
    { id: 'r2', name: 'kavach', remoteUrl: 'https://github.com/acme/kavach', provider: 'github', lastIndexedAt: '2026-03-10T00:00:00Z' },
    { id: 'r3', name: 'api-gateway', remoteUrl: 'https://github.com/acme/api-gateway', provider: 'github', lastIndexedAt: null },
  ],
  stats: { totalSessions: 142, totalCostUsd: 18.45, avgFtrScore: 0.73 }
});
