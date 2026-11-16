import { error } from '@sveltejs/kit';

/** The preview's module — its catalogue and pages — in development only. A production build
 *  answers 404 and bundles none of it: the import sits behind the build-time DEV constant,
 *  so the chunk drops, page markup included. */
export async function previewKit(): Promise<typeof import('./dev.js')> {
  const kit = import.meta.env.DEV ? await import('./dev.js') : null;
  if (!kit) error(404, 'the preview is a development page');
  return kit;
}
