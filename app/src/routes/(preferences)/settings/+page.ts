import { redirect } from '@sveltejs/kit';

/** `/settings` has no content of its own — the rail is the surface, and the
 *  active sub-route provides the content. Redirect to General so the rail
 *  always has a highlighted entry. */
export function load(): never {
  throw redirect(307, '/settings/general');
}
