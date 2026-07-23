import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { YOU_SECTIONS, labelForSection } from '$lib/dojo2-nav';

// A personal-zone placeholder for every NAV_YOU destination not yet ported. An
// unknown section (not a real nav destination) redirects to the landing so the
// two-nav never dead-ends on a fabricated path.
export const load: PageLoad = ({ params }) => {
	if (!YOU_SECTIONS.includes(params.section)) redirect(307, '/you');
	return { section: params.section, title: labelForSection(params.section, 'you') };
};
