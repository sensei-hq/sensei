import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

// #133: a kanji written as a bare `class="kanji"` element never sees the glyph
// preference. Every mark goes through <Kanji>, which can render its icon; only
// Kanji itself and ProjectGlyph (a project identity with an image fallback)
// write the class directly.
const ALLOWED = ['lib/components/Kanji.svelte', 'projects/ProjectGlyph.svelte'];

function svelteFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return svelteFiles(p);
    return p.endsWith('.svelte') && !p.includes('.harness.') ? [p] : [];
  });
}

describe('kanji marks', () => {
  it('render through <Kanji>, not a bare class="kanji" element', () => {
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
