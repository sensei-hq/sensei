import type { PageLoad } from './$types.js';
import { componentsOf } from '$lib/preview/catalogue.js';
import { previewKit } from '$lib/preview/load.js';

// The development preview's index (#429): every component in the catalogue, each a link
// to its own page. The page itself comes from the dev-only module.
export const load: PageLoad = async () => {
  const kit = await previewKit();
  return { page: kit.PreviewIndex, components: componentsOf(kit.CATALOGUE) };
};
