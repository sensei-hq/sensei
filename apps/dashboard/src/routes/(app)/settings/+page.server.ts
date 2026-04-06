import type { PageServerLoad } from './$types';

export const load: PageServerLoad = () => ({
  user: { id: 'u1', name: 'Jerry Thomas', email: 'jerry@acme.com', role: 'admin', avatarUrl: null, createdAt: '2025-01-01T00:00:00Z' },
  orgs: [
    { id: 'o1', name: 'Acme Corp', role: 'admin', plan: 'team' },
    { id: 'o2', name: 'Personal', role: 'owner', plan: 'free' },
  ],
  apiToken: { lastGenerated: '2026-01-15T00:00:00Z', prefix: 'snsi_****4a2f' },
  daemonStatus: { running: true, port: 51789, uptimeSeconds: 86400, version: '0.3.1' }
});
