// The shared family (@sensei/ui) under the dōjō's own build: the dōjō renders
// its errors, empties and strips through the same components as the desktop app
// (whose spec covers every variation). This checks they mount here, with the
// dōjō's rokkit and glyphs.
import { describe, it, expect, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import { Notice, EmptyState, Banner, loadFailedNotice } from '@sensei/ui';

afterEach(() => cleanup());

describe('@sensei/ui in the dōjō', () => {
	it('a failed read is a fault Notice panel, with a next step', () => {
		const { container, getByRole } = render(Notice, { props: { size: 'panel', ...loadFailedNotice('the triage queue', 'forbidden') } });
		const n = container.querySelector('[data-component="notice"]') as HTMLElement;
		expect(n.dataset.tone).toBe('fault');
		expect(getByRole('alert')).toBe(n);
		expect(n.querySelector('[data-part="remedy"]')!.textContent).toContain('Retry');
	});

	it('an honest empty is a status card, not an alert', () => {
		const { container } = render(EmptyState, { props: { kanji: '静', title: 'Nothing needs you.' } });
		const e = container.querySelector('[data-component="empty-state"]') as HTMLElement;
		expect(e.getAttribute('role')).toBe('status');
		expect(e.dataset.size).toBe('panel');
	});

	it('a strip takes a Notice tone', () => {
		const { container } = render(Banner, { props: { tone: 'refusal', title: 'Two scopes have no owner' } });
		expect((container.querySelector('[data-component="banner"]') as HTMLElement).dataset.tone).toBe('refusal');
	});
});
