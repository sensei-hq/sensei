import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// Legacy relay inbox. The run list moved to the personal Inbox (F2); old push
// notifications and bookmarks land here, so permanently redirect them to /you.
export const load: PageLoad = () => {
	redirect(308, '/you');
};
