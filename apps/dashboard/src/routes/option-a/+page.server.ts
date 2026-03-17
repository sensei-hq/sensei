import type { PageServerLoad } from './$types';
import { demoUser, demoWorkspaces, demoRepos, demoTeams, demoLibraries, demoSessions } from '$lib/demo-stubs';

export const load: PageServerLoad = () => ({
  user: demoUser,
  workspaces: demoWorkspaces,
  repos: demoRepos,
  teams: demoTeams,
  libraries: demoLibraries,
  sessions: demoSessions,
});
