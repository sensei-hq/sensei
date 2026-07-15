import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import Confidence from './Confidence.svelte';
import EnsoRing from './EnsoRing.svelte';
import ConsoleHead from './ConsoleHead.svelte';

// Render smoke tests for the console visual primitives — they mount and show the
// expected value with an injected fixture (incl. the null/unscored edge), so a
// screen built on them renders rather than crashing without a live backend.
describe('console primitives render', () => {
	afterEach(cleanup);

	it('Confidence shows the rounded percent', () => {
		const { getByText } = render(Confidence, { value: 0.84 });
		expect(getByText('84')).toBeTruthy();
	});

	it('Confidence shows a dash for an unscored (null) value', () => {
		const { getByText } = render(Confidence, { value: null });
		expect(getByText('—')).toBeTruthy();
	});

	it('EnsoRing renders its centred label', () => {
		const { getByText } = render(EnsoRing, { progress: 0.5, label: '50' });
		expect(getByText('50')).toBeTruthy();
	});

	it('ConsoleHead renders eyebrow + title', () => {
		const { getByText } = render(ConsoleHead, {
			kanji: '全',
			eyebrow: 'Acme · Dōjō',
			title: 'The shared mind, governed.'
		});
		expect(getByText('Acme · Dōjō')).toBeTruthy();
		expect(getByText('The shared mind, governed.')).toBeTruthy();
	});
});
