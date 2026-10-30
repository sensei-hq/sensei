// The diagrams' shared header (M17): four toggles, each opening one band — the
// info (use it to · how to read it · what to do), the workspace's numbers, the
// controls — and the side panel beside the diagram. All open by default; closing
// any is remembered.
import { STORAGE_KEYS, browserStorage } from '$lib/storage-keys.js';

const shown = (key: string) => browserStorage()?.getItem(key) !== 'closed';
const remember = (key: string, open: boolean) => browserStorage()?.setItem(key, open ? 'open' : 'closed');

export class DiagramHeaderState {
  open = $state(shown(STORAGE_KEYS.diagramsAbout));

  stats = $state(shown(STORAGE_KEYS.diagramsStats));
  controls = $state(shown(STORAGE_KEYS.diagramsControls));
  side = $state(shown(STORAGE_KEYS.diagramsSide));

  toggle(): void {
    this.open = !this.open;
    remember(STORAGE_KEYS.diagramsAbout, this.open);
  }

  toggleStats(): void {
    this.stats = !this.stats;
    remember(STORAGE_KEYS.diagramsStats, this.stats);
  }

  toggleControls(): void {
    this.controls = !this.controls;
    remember(STORAGE_KEYS.diagramsControls, this.controls);
  }

  toggleSide(): void {
    this.side = !this.side;
    remember(STORAGE_KEYS.diagramsSide, this.side);
  }
}
