// The fake crew driver (#426): lanes played over the lane HTTP API, the TypeScript twin
// of scripts/relay-queue-smoke.sh. The journey spec registers a coordinator and
// contributors with it, then claims, checkpoints, asks, reports and reviews as a crew
// would, while the app shows each change from the stream. A refusal is never swallowed:
// it throws a RelayCallError with the daemon's status and body, or — where the journey
// expects one — `attempt()` returns it.

/** The daemon refused a lane call. */
export class RelayCallError extends Error {
  constructor(
    readonly path: string,
    readonly status: number,
    readonly body: unknown,
    readonly method = 'POST',
  ) {
    super(`${method} ${path}: HTTP ${status}: ${JSON.stringify(body)}`);
  }
}

export interface RegisterBody {
  family: string;
  role?: 'coordinator' | 'contributor';
  suggested_name?: string;
  crew_id?: string;
  goal?: string;
  repo_key?: string;
  model?: string;
  version?: string;
  capabilities?: string[];
  /** The editor the session runs in (#396). */
  host?: string;
}

interface Registered {
  lane_id: string;
  lane_token: string;
  crew_id: string;
  lane_title?: string | null;
  suggested_name?: string | null;
}

export interface Step {
  key: string;
  title: string;
  state: 'open' | 'done' | 'skipped';
  reason?: string;
}

export interface ReportBody {
  phase: string;
  round: number;
  summary: { changed: string };
  branch?: string;
  report_md?: string;
  partial?: boolean;
  files?: Array<{ path: string; added: number; removed: number }>;
}

export interface Claim {
  claim_type: 'task' | 'review' | 'none';
  task?: { id: string; title?: string; deps_status?: string };
}

/** A decision answer in the envelope: a pick, or a form's values (#389). */
export interface EnvelopeAnswer {
  id: string;
  decision_id: string;
  task_id: string | null;
  option: string | null;
  values: Record<string, unknown> | null;
  note: string | null;
}

/** The lane envelope (`GET /api/relay/tasks`); the fields the journeys read. */
export interface Envelope {
  messages: unknown[];
  answers: EnvelopeAnswer[];
  reviews: unknown[];
  assignments: unknown[];
}

type Fetch = typeof globalThis.fetch;

async function call<T>(fetch: Fetch, base: string, path: string, token: string | null, body: unknown): Promise<T> {
  const res = await fetch(`${base}${path}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...(token ? { Authorization: `Bearer ${token}` } : {}) },
    body: JSON.stringify(body),
  });
  const text = await res.text();
  const parsed: unknown = text ? JSON.parse(text) : null;
  if (!res.ok) throw new RelayCallError(path, res.status, parsed);
  return parsed as T;
}

/** The crew's front door: registration. */
export class RelayCrew {
  constructor(
    readonly base: string,
    readonly fetch: Fetch = globalThis.fetch,
  ) {}

  async register(body: RegisterBody): Promise<RelayLane> {
    const registered = await call<Registered>(this.fetch, this.base, '/api/relay/register', null, body);
    return new RelayLane(this, registered);
  }
}

/** One registered lane: every call signed with its token and naming its lane id. */
export class RelayLane {
  readonly id: string;
  readonly crewId: string;
  readonly name: string | null;
  readonly #token: string;

  constructor(
    readonly crew: RelayCrew,
    registered: Registered,
  ) {
    this.id = registered.lane_id;
    this.crewId = registered.crew_id;
    this.name = registered.suggested_name ?? registered.lane_title ?? null;
    this.#token = registered.lane_token;
  }

  #post<T>(path: string, body: Record<string, unknown> = {}): Promise<T> {
    return call<T>(this.crew.fetch, this.crew.base, path, this.#token, { lane_id: this.id, ...body });
  }

  heartbeat(): Promise<unknown> {
    return this.#post('/api/relay/heartbeat');
  }

  claimNext(): Promise<Claim> {
    return this.#post<Claim>('/api/relay/tasks/claim_next');
  }

  checkpoint(taskId: string, body: { current_step?: string; notes?: string; steps?: Step[] }): Promise<unknown> {
    return this.#post(`/api/relay/tasks/${taskId}/checkpoint`, body);
  }

  /** Plan a task on the crew's board (coordinator); its id. */
  async plan(body: { title: string; phase: string; repo_key: string }): Promise<string> {
    const planned = await this.#post<{ task_id: string }>('/api/relay/tasks/plan', body);
    return planned.task_id;
  }

  /** Ask the person: buttons (`options`) or a form (`response`, #389), never both. */
  async ask(body: {
    task_id?: string;
    question: string;
    options?: Array<{ id: string; label: string; detail?: string }>;
    recommended?: string;
    response?: unknown;
    context?: string;
  }): Promise<string> {
    const created = await this.#post<{ decision_id: string }>('/api/relay/decisions', { asked_of: 'person', ...body });
    return created.decision_id;
  }

  /** The lane's envelope, now: what is addressed to it, answers to its asks included. */
  async envelope(): Promise<Envelope> {
    const path = `/api/relay/tasks?lane_id=${encodeURIComponent(this.id)}`;
    const res = await this.crew.fetch(`${this.crew.base}${path}`, { method: 'GET', headers: { Authorization: `Bearer ${this.#token}` } });
    const text = await res.text();
    const parsed: unknown = text ? JSON.parse(text) : null;
    if (!res.ok) throw new RelayCallError(path, res.status, parsed, 'GET');
    return parsed as Envelope;
  }

  report(taskId: string, body: ReportBody): Promise<{ state?: string }> {
    return this.#post(`/api/relay/tasks/${taskId}/report`, { ...body });
  }

  /** Task 0's report: the plan as a ```json block in report_md, which the broker unpacks. */
  reportPlan(taskId: string, plan: { tasks: unknown[] }, changed: string): Promise<{ state?: string }> {
    const report_md = `Plan:\n\n\`\`\`json\n${JSON.stringify(plan)}\n\`\`\`\n`;
    return this.report(taskId, { phase: 'planning', round: 1, summary: { changed }, report_md });
  }

  claimReview(taskId: string): Promise<unknown> {
    return this.#post(`/api/relay/tasks/${taskId}/claim_review`);
  }

  review(taskId: string, verdict: 'approved' | 'rework', summary: string): Promise<{ state?: string }> {
    return this.#post(`/api/relay/tasks/${taskId}/review`, { verdict, summary });
  }

  unregister(): Promise<unknown> {
    return this.#post('/api/relay/unregister');
  }

  /** Close the lane's crew (coordinator): the lanes it released with no live lease (#398). */
  closeCrew(): Promise<{ crew_id: string; status: string; released: string[] }> {
    return this.#post(`/api/relay/crews/${this.crewId}/close`);
  }

  /** Run a call the journey expects may be refused: its result, or the refusal. */
  static async attempt<T>(
    run: () => Promise<T>,
  ): Promise<{ ok: true; value: T } | { ok: false; status: number; error: string | null; body: unknown }> {
    try {
      return { ok: true, value: await run() };
    } catch (e) {
      if (!(e instanceof RelayCallError)) throw e;
      const error = (e.body as { error?: unknown } | null)?.error;
      return { ok: false, status: e.status, error: typeof error === 'string' ? error : null, body: e.body };
    }
  }
}
