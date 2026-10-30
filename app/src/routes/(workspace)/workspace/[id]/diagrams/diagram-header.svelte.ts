// The diagrams' shared header (M17): an info toggle opens what the view shows,
// what to use it for and how to read it; a stats toggle, the workspace's numbers
// every diagram shares. Both open by default; closing either is remembered.
import { STORAGE_KEYS, browserStorage } from '$lib/storage-keys.js';

export class DiagramHeaderState {
  open = $state(browserStorage()?.getItem(STORAGE_KEYS.diagramsAbout) !== 'closed');

  stats = $state(browserStorage()?.getItem(STORAGE_KEYS.diagramsStats) !== 'closed');

  toggle(): void {
    this.open = !this.open;
    browserStorage()?.setItem(STORAGE_KEYS.diagramsAbout, this.open ? 'open' : 'closed');
  }

  toggleStats(): void {
    this.stats = !this.stats;
    browserStorage()?.setItem(STORAGE_KEYS.diagramsStats, this.stats ? 'open' : 'closed');
  }
}
