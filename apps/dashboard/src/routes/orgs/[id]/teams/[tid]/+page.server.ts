import type { PageServerLoad } from './$types';

export const load: PageServerLoad = () => ({
  org: { id: 'o1', name: 'Acme Corp' },
  team: { id: 't1', name: 'Platform', description: 'Core infrastructure team', createdAt: '2025-01-20T00:00:00Z' },
  members: [
    { id: 'u1', name: 'Jerry Thomas', email: 'jerry@acme.com', role: 'lead' },
    { id: 'u4', name: 'Priya Patel', email: 'priya@acme.com', role: 'member' },
    { id: 'u5', name: 'Marcus Johnson', email: 'marcus@acme.com', role: 'member' },
    { id: 'u6', name: 'Lisa Wong', email: 'lisa@acme.com', role: 'member' },
  ],
  repos: [
    { id: 'r1', name: 'sensei', remoteUrl: 'https://github.com/acme/sensei', lastIndexedAt: '2026-03-17T00:00:00Z' },
    { id: 'r3', name: 'api-gateway', remoteUrl: 'https://github.com/acme/api-gateway', lastIndexedAt: null },
    { id: 'r4', name: 'platform-core', remoteUrl: 'https://github.com/acme/platform-core', lastIndexedAt: '2026-03-15T00:00:00Z' },
  ]
});
