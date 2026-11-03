// A read that failed (#335), said through the shared Notice
// (docs/architecture/notice.md) so it reads the same as every error in sensei.
import { describe, it, expect, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import LoadError from './LoadError.svelte';

afterEach(() => cleanup());

describe('LoadError', () => {
	it('is a fault Notice panel, announced to assistive tech', () => {
		const { container, getByRole } = render(LoadError, { props: { what: 'the triage queue', error: 'forbidden' } });
		const n = container.querySelector('[data-component="notice"]') as HTMLElement;
		expect(n.dataset.size).toBe('panel');
		expect(n.dataset.tone).toBe('fault');
		expect(getByRole('alert')).toBe(n);
	});

	it('names what failed, then why', () => {
		const { container } = render(LoadError, { props: { what: 'the triage queue', error: 'forbidden' } });
		expect(container.querySelector('[data-part="heading"]')!.textContent).toBe("Couldn't load the triage queue");
		expect(container.querySelector('[data-part="detail"]')!.textContent).toBe('forbidden');
	});

	it('offers Retry only when the screen can retry', () => {
		const none = render(LoadError, { props: { what: 'x', error: 'y' } });
		expect(none.container.querySelector('[data-action="retry"]')).toBeNull();
		cleanup();
		let tried = 0;
		const some = render(LoadError, { props: { what: 'x', error: 'y', onretry: () => tried++ } });
		(some.container.querySelector('[data-action="retry"]') as HTMLButtonElement).click();
		expect(tried).toBe(1);
	});

	it('with no reason given, still says something', () => {
		const { container } = render(LoadError, { props: { what: 'your metrics', error: '' } });
		expect(container.querySelector('[data-part="detail"]')!.textContent).toBe('The dōjō gave no answer.');
	});
});
