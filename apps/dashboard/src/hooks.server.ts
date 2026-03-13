import { kavach } from '$kavach/auth';

export const handle = ({ event, resolve }: Parameters<typeof import('@sveltejs/kit').Handle>[0]) =>
  kavach.handle({ event, resolve });
