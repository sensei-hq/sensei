// GET/PUT /v1/you/preferences — the signed-in person's display preferences
// (#133): the glyph mode and when it was chosen. Read and written as the person
// (RLS keeps the row theirs); the desktop app's daemon syncs through it, and the
// later choice wins. A PUT answers what is stored, so a caller holding an older
// choice learns the newer one.
import type { RequestHandler } from './$types';
import { callerDb } from '$lib/server/caller-db';
import { apiError, resolveCaller } from '$lib/server/dojo-auth';
import { readPreferences, writeGlyphs } from '$lib/server/preferences';

export const GET: RequestHandler = async ({ request, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		return Response.json(await readPreferences(callerDb(caller), caller.userId));
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};

export const PUT: RequestHandler = async ({ request, locals }) => {
	try {
		const caller = await resolveCaller(request, locals);
		const body = (await request.json().catch(() => ({}))) as Record<string, unknown>;
		const glyphs = body.glyphs;
		const at = body.glyphs_set_at;
		if (glyphs !== 'kanji' && glyphs !== 'icon') return apiError(400, 'glyphs must be kanji or icon');
		if (typeof at !== 'string' || Number.isNaN(Date.parse(at))) {
			return apiError(400, 'glyphs_set_at must be the time the choice was made');
		}
		return Response.json(await writeGlyphs(callerDb(caller), caller.userId, { glyphs, glyphs_set_at: at }));
	} catch (e) {
		if (e instanceof Response) return e;
		throw e;
	}
};
