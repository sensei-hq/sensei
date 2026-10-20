import { describe, it, expect } from 'vitest';
import { contentInclude } from '../../uno.content.js';

// #133: the rails' Solar icon classes live in `.ts` data files. UnoCSS's default
// scan skips plain `.ts`, so a production build shipped no CSS for them — icon
// mode rendered empty slots while every unit test passed.
describe('uno.config content pipeline', () => {
  const scans = (path: string) => contentInclude.some((r) => r.test(path));

  it('scans the .ts data files that carry icon classes', () => {
    expect(scans('/app/src/routes/(observatory)/observatory-nav.ts')).toBe(true);
    expect(scans('/app/src/routes/(observatory)/settings/settings-nav.ts')).toBe(true);
  });

  it('still scans components', () => {
    expect(scans('/app/src/lib/components/PageHeader.svelte')).toBe(true);
  });
});
