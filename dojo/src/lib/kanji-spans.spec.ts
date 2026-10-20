import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

// #133: a kanji written as a bare `class="kanji"` element never sees the glyph
// choice. Every dōjō mark goes through KanjiToken, which can render its icon;
// only KanjiToken itself, and NavPane / TabBar (which render navMark's choice),
// write the class directly.
const ALLOWED = ['kit/KanjiToken.svelte', 'kit/NavPane.svelte', 'kit/TabBar.svelte'];

function svelteFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return svelteFiles(p);
    return p.endsWith('.svelte') && !p.includes('.harness.') ? [p] : [];
  });
}

describe('kanji marks', () => {
  it('render through KanjiToken, not a bare class="kanji" element', () => {
    const bare: string[] = [];
    for (const file of svelteFiles('src')) {
      if (ALLOWED.some((a) => file.endsWith(a))) continue;
      readFileSync(file, 'utf8')
        .split('\n')
        .forEach((line, i) => {
          if (/class="kanji\b/.test(line)) bare.push(`${file.replace('src/', '')}:${i + 1}`);
        });
    }
    expect(bare).toEqual([]);
  });
});
