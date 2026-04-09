import { json } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = () =>
  json({ summary: { totalSymbols: 0, totalEdges: 0, communities: 0 }, projects: [], communities: [], godNodes: [], rationale: [] });
