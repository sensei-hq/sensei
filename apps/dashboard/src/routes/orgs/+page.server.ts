import type { PageServerLoad } from './$types';

export const load: PageServerLoad = () => ({
  orgs: [
    { id: 'o1', name: 'Acme Corp', slug: 'acme', plan: 'team', memberCount: 12, repoCount: 8, description: 'Main company workspace' },
    { id: 'o2', name: 'Personal', slug: 'jerry', plan: 'free', memberCount: 1, repoCount: 4, description: 'Personal projects' },
    { id: 'o3', name: 'Open Source Labs', slug: 'oslabs', plan: 'free', memberCount: 6, repoCount: 11, description: 'Community open source projects' },
  ]
});
