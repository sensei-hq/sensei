import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ locals, url }) => {
  const redirectUri = url.searchParams.get('redirect_uri');
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const session = locals.session as any;
  const accessToken: string | undefined = session?.access_token;

  if (!redirectUri) {
    return { error: 'Missing redirect_uri parameter', loggedIn: !!accessToken };
  }

  if (!accessToken) {
    // Not logged in — redirect to auth, come back here after
    const returnTo = `/connect/cli?redirect_uri=${encodeURIComponent(redirectUri)}`;
    throw redirect(302, `/auth?returnTo=${encodeURIComponent(returnTo)}`);
  }

  // Logged in — send access token back to CLI callback
  throw redirect(302, `${redirectUri}?access_token=${encodeURIComponent(accessToken)}`);
};
