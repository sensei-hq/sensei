import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { SHORTCUTS } from './shortcuts.js';

// #135: the help page listed ⌘N, ⌘1–3, ⌘\, ⌘0 and ⌘? — none of them bound, so
// it sent people to keys that did nothing. Its table now renders SHORTCUTS, and
// every entry is held to where the binding actually lives.
const libRs = readFileSync(resolve(__dirname, '../../src-tauri/src/lib.rs'), 'utf8');
const accelerators = new Set([...libRs.matchAll(/\.accelerator\("([^"]+)"\)/g)].map((m) => m[1]));
const layout = readFileSync(resolve(__dirname, '../routes/(observatory)/+layout.svelte'), 'utf8');

describe('SHORTCUTS', () => {
  it('lists only keys that are bound, where they are bound', () => {
    for (const s of SHORTCUTS) {
      if (s.source === 'menu') expect(accelerators, s.action).toContain(s.binding);
      else expect(layout, s.action).toContain(`shortcut: '${s.binding}'`);
    }
  });

  it('documents every menu accelerator', () => {
    const listed = new Set(SHORTCUTS.filter((s) => s.source === 'menu').map((s) => s.binding));
    for (const a of accelerators) expect(listed, `${a} is bound but not on the help page`).toContain(a);
  });
});
