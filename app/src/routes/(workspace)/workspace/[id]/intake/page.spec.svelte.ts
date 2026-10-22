// @vitest-environment jsdom
import { describe, it, expect, afterEach } from 'vitest';
import { mountComponent } from '$lib/test-mount.js';
import Page from './+page.svelte';
import type { PageData } from './$types.js';

let cleanup: Array<() => void> = [];
afterEach(() => { cleanup.forEach((fn) => fn()); cleanup = []; });

// #288: a guide that could not be read is an error with retry — not an intake
// form with no frame, no questions and no playbooks to recommend from.
describe('Intake /+page — a failed guide read', () => {
  it('shows the error, and no form', () => {
    const m = mountComponent(Page, { data: { guide: null, guideError: 'Service Unavailable', workspaceId: 'p1' } as unknown as PageData });
    cleanup.push(m.destroy);
    expect(m.container.querySelector('[data-screen-state="error"]')?.textContent).toContain('Service Unavailable');
    expect(m.container.querySelector('textarea, form')).toBeNull();
  });
});
