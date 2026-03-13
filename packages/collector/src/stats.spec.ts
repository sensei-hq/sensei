import { describe, it, expect, vi } from "vitest";
import { queryStats } from "./stats.js";

const now = Date.now();
const day = 24 * 60 * 60 * 1000;
const recent = now - 3 * day;
const old = now - 10 * day;

// Events as they come from Supabase (ts is ISO string, success is boolean)
const allEvents = [
  { id: "1", user_uuid: "u1", session_id: "sess-a", tool: "search_index", phase: "post", duration_ms: 120, success: true, ts: new Date(recent).toISOString(), project_path: "/proj", input: null, error: null },
  { id: "2", user_uuid: "u1", session_id: "sess-a", tool: "search_index", phase: "post", duration_ms: 200, success: true, ts: new Date(recent + 1000).toISOString(), project_path: "/proj", input: null, error: null },
  { id: "3", user_uuid: "u1", session_id: "sess-a", tool: "Bash", phase: "post", duration_ms: 300, success: false, ts: new Date(recent + 2000).toISOString(), project_path: "/proj", input: null, error: "exit 1" },
  { id: "4", user_uuid: "u1", session_id: "sess-b", tool: "Read", phase: "post", duration_ms: 50, success: true, ts: new Date(recent + 3000).toISOString(), project_path: "/proj", input: null, error: null },
  { id: "5", user_uuid: "u1", session_id: "sess-c", tool: "search_index", phase: "post", duration_ms: 100, success: true, ts: new Date(old).toISOString(), project_path: "/proj", input: null, error: null },
];

function makeMockClient(eventsToReturn: any[]) {
  const result = Promise.resolve({ data: eventsToReturn, error: null });
  const chain: any = {
    select: () => chain,
    eq: () => chain,
    neq: () => chain,
    gte: () => chain,
    order: () => chain,
    not: () => chain,
    then: result.then.bind(result),
    catch: result.catch.bind(result),
  };
  return { from: () => chain } as any;
}

describe("queryStats", () => {
  it("default 7-day summary counts only recent events", async () => {
    // Return only recent events (simulate Supabase filtering by ts >= 7 days ago)
    const recentEvents = allEvents.filter(e => new Date(e.ts).getTime() >= now - 7 * day);
    const client = makeMockClient(recentEvents);
    const result = await queryStats(client, {});
    expect(result.total_calls).toBe(4);
    expect(result.tools.some(t => t.name === "search_index")).toBe(true);
    expect(result.sessions).toBe(2);
  });

  it("--all includes all events", async () => {
    const client = makeMockClient(allEvents);
    const result = await queryStats(client, { all: true });
    expect(result.total_calls).toBe(5);
  });

  it("--tool returns stats for one tool", async () => {
    const searchEvents = allEvents.filter(e => e.tool === "search_index" && new Date(e.ts).getTime() >= now - 7 * day);
    const client = makeMockClient(searchEvents);
    const result = await queryStats(client, { tool: "search_index" });
    expect(result.tool).toBeDefined();
    expect(result.tool!.name).toBe("search_index");
    expect(result.tool!.calls).toBe(2);
    expect(result.tool!.success_rate).toBe(1.0);
    expect(result.tool!.avg_duration_ms).toBe(160);
  });

  it("--session returns chronological events", async () => {
    const sessEvents = allEvents.filter(e => e.session_id === "sess-a");
    const client = makeMockClient(sessEvents);
    const result = await queryStats(client, { session: "sess-a" });
    expect(result.events).toBeDefined();
    expect(result.events!.length).toBe(3);
    // ts should be a number in the result
    expect(typeof result.events![0].ts).toBe("number");
  });

  it("tools list is sorted by call count descending", async () => {
    const recentEvents = allEvents.filter(e => new Date(e.ts).getTime() >= now - 7 * day);
    const client = makeMockClient(recentEvents);
    const result = await queryStats(client, {});
    for (let i = 1; i < result.tools.length; i++) {
      expect(result.tools[i - 1].calls).toBeGreaterThanOrEqual(result.tools[i].calls);
    }
  });
});
