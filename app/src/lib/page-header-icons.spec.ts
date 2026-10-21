import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

// #133: every screen header that shows a kanji also names the icon that stands
// for it, so icon mode never leaves a screen with a kanji-only mark.
function svelteFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return svelteFiles(p);
    return p.endsWith('.svelte') && !p.includes('.harness.') ? [p] : [];
  });
}

describe('PageHeader marks', () => {
  it('gives every header with a kanji an icon', () => {
    const missing: string[] = [];
    let headers = 0;
    for (const file of svelteFiles('src/routes')) {
      for (const m of readFileSync(file, 'utf8').matchAll(/<PageHeader\b([\s\S]*?)\/?>/g)) {
        if (!/\bkanji=/.test(m[1])) continue;
        headers++;
        if (!/\bicon=/.test(m[1])) missing.push(file.replace('src/routes/', ''));
      }
    }
    expect(headers).toBeGreaterThan(20);
    expect(missing).toEqual([]);
  });
});
