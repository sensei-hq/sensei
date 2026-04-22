/**
 * @deprecated Import from './projects.svelte.js' instead.
 * This file re-exports everything for backwards compatibility during migration.
 */
export {
  getProjects as getSolutions,
  getActiveProjectId as getActiveSolutionId,
  isProjectsLoaded as isSolutionsLoaded,
  markLoaded,
  setActiveProjectId as setActiveSolutionId,
  loadProjects as loadSolutions,
  createProject as createSolution,
  updateProject as updateSolution,
  deleteProject as deleteSolution,
  addRepoToProject as addRepoToSolution,
  removeRepoFromProject as removeRepoFromSolution,
  getProjectById as getSolutionById,
  getProjectForRepo as getSolutionForRepo,
  getProjectsByCategory as getSolutionsByCategory,
  inferRepoRole,
  clearAllProjects as clearAllSolutions,
} from './projects.svelte.js';

/** @deprecated */
export function getStandaloneLibraries() { return []; }
