import { writable } from 'svelte/store';

export const currentProject = writable<string | null>(null);
export const sidebarOpen = writable(true);

export function resetState() {
  currentProject.set(null);
  sidebarOpen.set(true);
}
