import { describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import { CodeMapHost } from './codemap-host.svelte.js';
import type { CodeMapPort } from '@sensei/codemap';

describe('CodeMapHost', () => {
  // SvelteKit replaces `page.params` on every navigation, the same id included: a
  // lens switch must keep the map, and only another workspace gets a new one.
  it('keeps one map while the workspace id stays, and a new one for another', () => {
    const page = $state({ params: { id: 'w1' } });
    const host = new CodeMapHost(() => page.params.id, {} as CodeMapPort);
    const first = host.state;
    page.params = { id: 'w1' };
    flushSync();
    expect(host.state).toBe(first);
    page.params = { id: 'w2' };
    flushSync();
    expect(host.state).not.toBe(first);
  });
});
