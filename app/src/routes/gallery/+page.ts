import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.js';

// The component gallery is a development page: every relay component rendered
// from its fixture catalogue, for review against the mockup and for the capture
// matrix (e2e/gallery). A production build answers 404 and never bundles the
// catalogue — the import sits behind the build-time DEV constant.
export const load: PageLoad = async ({ url }) => {
  // A constant condition on the import itself: a production build drops the
  // catalogue chunk entirely, not only the route's answer.
  const catalogue = import.meta.env.DEV ? await import('$lib/relay/gallery.js') : null;
  if (!catalogue) error(404, 'the gallery is a development page');
  const { GALLERY } = catalogue;
  const pick = (key: string, allowed: readonly string[], dflt: string) => {
    const v = url.searchParams.get(key);
    return v && allowed.includes(v) ? v : dflt;
  };
  return {
    entries: GALLERY,
    theme: pick('theme', ['light', 'dark'], 'light') as 'light' | 'dark',
    glyph: pick('glyph', ['kanji', 'icon'], 'kanji') as 'kanji' | 'icon',
    width: pick('width', ['wide', 'phone'], 'wide') as 'wide' | 'phone',
  };
};
