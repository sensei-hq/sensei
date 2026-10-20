// The keyboard shortcuts the help page documents (#135). Each names where it is
// BOUND — a native menu accelerator in src-tauri/src/lib.rs, or a command
// shortcut in the observatory layout — and shortcuts.spec.ts holds every entry
// to that binding, so the page cannot promise a key that does nothing again.
export interface Shortcut {
  /** As shown to a person. */
  keys: string;
  action: string;
  source: 'menu' | 'command';
  /** The binding string as written at its source. */
  binding: string;
}

export const SHORTCUTS: Shortcut[] = [
  { keys: '⌘K', action: 'Command palette', source: 'command', binding: 'mod+k' },
  { keys: '⌘⇧I', action: 'Identities', source: 'menu', binding: 'CmdOrCtrl+Shift+I' },
  { keys: '⌘⇧L', action: 'Diagnostic logs', source: 'menu', binding: 'CmdOrCtrl+Shift+L' },
  { keys: '⌘?', action: 'Keyboard shortcuts (this page)', source: 'menu', binding: 'CmdOrCtrl+Shift+/' },
];
