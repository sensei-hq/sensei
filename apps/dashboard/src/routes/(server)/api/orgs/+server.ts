import { json, error } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, fetch, locals }) => {
  const userId = locals.session?.user?.id;
  if (!userId) throw error(401, 'Not authenticated');

  const body = await request.json() as { name: string; slug: string };
  if (!body.name?.trim()) throw error(400, 'Name is required');
  if (!body.slug?.trim()) throw error(400, 'Slug is required');

  // Create account
  const accountRes = await fetch('/data/core/accounts', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Prefer: 'return=representation' },
    body: JSON.stringify({ display_name: body.name.trim(), slug: body.slug.trim(), account_type: 'team' }),
  });
  if (!accountRes.ok) {
    const err = await accountRes.json();
    throw error(400, err?.message ?? 'Failed to create organization');
  }
  const [account] = await accountRes.json();

  // Link creator as owner
  const memberRes = await fetch('/data/core/profile_accounts', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ user_id: userId, account_id: account.id, role: 'owner' }),
  });
  if (!memberRes.ok) throw error(500, 'Failed to link you as owner');

  return json({ id: account.id, slug: account.slug, name: account.display_name });
};
