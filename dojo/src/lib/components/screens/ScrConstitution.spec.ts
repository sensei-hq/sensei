import { render, cleanup } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import ScrConstitution from './ScrConstitution.svelte';

// #106: your stance and your personal rules are kept by sensei on your machine
// (D25); the dōjō holds neither. The screen used to show a mockup person's
// ("Rin Saito") rules and dials as yours.
describe('ScrConstitution — what the dōjō does not hold, it says so', () => {
	afterEach(cleanup);

	it('with no stance and no personal rules, says where they live and shows none', () => {
		const { getByText, queryByText } = render(ScrConstitution, { props: {} });
		expect(getByText(/kept by sensei on your machine/)).toBeTruthy();
		expect(queryByText('autonomy')).toBeNull();
		expect(queryByText('Rin Saito')).toBeNull();
		expect(queryByText('Your effective constitution')).toBeNull();
	});
});
