import { describe, it, expect, beforeEach } from 'vitest';
import { isRedirect } from '@sveltejs/kit';
import { load } from './+layout.js';
import { healthState } from '$lib/health-state.svelte.js';
import { wizardState } from '$lib/wizard-state.svelte.js';

type Load = (event: { url: URL; depends: (dep: string) => void }) => unknown;

/** Where the root load sends `href`: the redirect's location, or undefined. */
async function gate(href: string): Promise<string | undefined> {
  try {
    await (load as unknown as Load)({ url: new URL(href), depends: () => {} });
    return undefined;
  } catch (e) {
    if (isRedirect(e)) return e.location;
    throw e;
  }
}

describe('root layout load — the boot gate', () => {
  beforeEach(() => {
    healthState.status = 'ok';
    wizardState.setupComplete = true;
  });

  it('redirects a gated route to /health while health is not ok', async () => {
    healthState.status = 'checking';
    expect(await gate('http://localhost/')).toBe('/health');
  });

  it('redirects to /setup/welcome once healthy but setup is incomplete', async () => {
    wizardState.setupComplete = false;
    expect(await gate('http://localhost/workspaces')).toBe('/setup/welcome');
  });

  it('lets a route through when both gates are open', async () => {
    expect(await gate('http://localhost/')).toBeUndefined();
  });

  // The bug this replaces: SvelteKit caches a `reroute` result per URL for the
  // session, so once `/` resolved to the setup screen it stayed there after
  // setup completed. A load is evaluated on every navigation — the same URL must
  // get a fresh answer when the state behind the gate has changed.
  it('answers the same URL afresh after the gate state changes', async () => {
    wizardState.setupComplete = false;
    expect(await gate('http://localhost/')).toBe('/setup/welcome');
    wizardState.setupComplete = true;
    expect(await gate('http://localhost/')).toBeUndefined();
  });
});
