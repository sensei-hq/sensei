import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.js';
import { componentsOf, entriesFor, readView } from '$lib/preview/catalogue.js';
import { previewKit } from '$lib/preview/load.js';

// One component's preview (#429): its fixtures and the view the query asks for. `all` is
// every fixture on one page, the one the capture matrix shoots. An unknown component is a
// 404, never an empty page. The page itself comes from the dev-only module.
export const load: PageLoad = async ({ params, url }) => {
  const kit = await previewKit();
  const entries = entriesFor(kit.CATALOGUE, params.component);
  if (entries.length === 0) error(404, `no component named ${params.component}`);
  return {
    page: kit.PreviewComponent,
    component: params.component,
    entries,
    components: componentsOf(kit.CATALOGUE),
    view: readView(url.searchParams),
  };
};
