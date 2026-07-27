import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// Legacy relay run detail. The run detail moved to /you/runs/<id> (F2/F3); old
// push-notification deep links point here, so permanently redirect them there.
export const load: PageLoad = ({ params }) => {
	redirect(308, `/you/runs/${params.run_id}`);
};
