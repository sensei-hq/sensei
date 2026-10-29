// The diagrams' shared header (M17): "What it's for" opens onto what the view
// shows, what to use it for and how to read it. Open by default; closing it is
// remembered, so a reader who knows the views is not shown the copy again.
import { STORAGE_KEYS, browserStorage } from '$lib/storage-keys.js';

export class DiagramHeaderState {
  open = $state(browserStorage()?.getItem(STORAGE_KEYS.diagramsAbout) !== 'closed');

  toggle(): void {
    this.open = !this.open;
    browserStorage()?.setItem(STORAGE_KEYS.diagramsAbout, this.open ? 'open' : 'closed');
  }
}
