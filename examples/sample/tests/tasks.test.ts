import { describe, it, expect, beforeEach } from "bun:test";
import app from "../src/index.js";
import { resetDb } from "../src/db.js";

beforeEach(() => resetDb());

async function req(method: string, path: string, body?: unknown) {
  return app.request(path, {
    method,
    headers: body ? { "Content-Type": "application/json" } : {},
    body: body ? JSON.stringify(body) : undefined,
  });
}

async function makeProject(name = "Test Project") {
  return (await req("POST", "/projects", { name })).json();
}

async function makeTask(projectId: string, overrides: Record<string, unknown> = {}) {
  return (
    await req("POST", "/tasks", { project_id: projectId, title: "Task", ...overrides })
  ).json();
}

// ── CRUD (complete — should pass) ─────────────────────────────────────────────

describe("POST /tasks", () => {
  it("creates a task with defaults", async () => {
    const p = await makeProject();
    const res = await req("POST", "/tasks", { project_id: p.id, title: "Do thing" });
    expect(res.status).toBe(201);
    const t = await res.json();
    expect(t.title).toBe("Do thing");
    expect(t.status).toBe("todo");
    expect(t.priority).toBe(2);
  });

  it("rejects unknown project_id", async () => {
    const res = await req("POST", "/tasks", {
      project_id: "00000000-0000-0000-0000-000000000000",
      title: "X",
    });
    expect(res.status).toBe(422);
  });

  it("rejects empty title", async () => {
    const p = await makeProject();
    const res = await req("POST", "/tasks", { project_id: p.id, title: "" });
    expect(res.status).toBe(400);
  });
});

describe("GET /tasks/:id", () => {
  it("returns the task", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    const res = await req("GET", `/tasks/${t.id}`);
    expect(res.status).toBe(200);
    expect((await res.json()).id).toBe(t.id);
  });

  it("returns 404 for unknown task", async () => {
    expect((await req("GET", "/tasks/00000000-0000-0000-0000-000000000000")).status).toBe(404);
  });
});

describe("PATCH /tasks/:id", () => {
  it("updates status", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    const res = await req("PATCH", `/tasks/${t.id}`, { status: "in-progress" });
    expect((await res.json()).status).toBe("in-progress");
  });

  it("updates priority", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    const res = await req("PATCH", `/tasks/${t.id}`, { priority: 1 });
    expect((await res.json()).priority).toBe(1);
  });
});

describe("DELETE /tasks/:id", () => {
  it("deletes the task", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    expect((await req("DELETE", `/tasks/${t.id}`)).status).toBe(204);
    expect((await req("GET", `/tasks/${t.id}`)).status).toBe(404);
  });
});

describe("GET /tasks", () => {
  it("filters by status", async () => {
    const p = await makeProject();
    await makeTask(p.id, { status: "todo" });
    const t2 = await makeTask(p.id, { status: "done" });
    const res = await req("GET", "/tasks?status=done");
    const body = await res.json();
    expect(body.data).toHaveLength(1);
    expect(body.data[0].id).toBe(t2.id);
  });

  it("filters by priority", async () => {
    const p = await makeProject();
    await makeTask(p.id, { priority: 1 });
    await makeTask(p.id, { priority: 3 });
    const body = await (await req("GET", "/tasks?priority=1")).json();
    expect(body.data).toHaveLength(1);
  });
});

// ── Feature stubs (should fail until features are implemented) ─────────────────

describe("GET /tasks?q= — full-text search (feature 2)", () => {
  it("returns results matching the query", async () => {
    const p = await makeProject();
    await makeTask(p.id, { title: "Fix authentication bug" });
    await makeTask(p.id, { title: "Update readme" });
    const res = await req("GET", "/tasks?q=authentication");
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.data).toHaveLength(1);
    expect(body.data[0].title).toBe("Fix authentication bug");
  });

  it("returns empty for no match", async () => {
    const p = await makeProject();
    await makeTask(p.id, { title: "Do something" });
    const res = await req("GET", "/tasks?q=xyznotfound");
    expect(res.status).toBe(200);
    expect((await res.json()).data).toHaveLength(0);
  });
});

describe("PATCH /tasks/bulk — bulk status update (feature 3)", () => {
  it("updates all given task ids to the target status", async () => {
    const p = await makeProject();
    const t1 = await makeTask(p.id);
    const t2 = await makeTask(p.id);
    const res = await req("PATCH", "/tasks/bulk", { ids: [t1.id, t2.id], status: "done" });
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.updated).toBe(2);
    expect((await req("GET", `/tasks/${t1.id}`).then(r => r.json())).status).toBe("done");
    expect((await req("GET", `/tasks/${t2.id}`).then(r => r.json())).status).toBe("done");
  });

  it("collects not-found ids rather than throwing", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    const res = await req("PATCH", "/tasks/bulk", {
      ids: [t.id, "00000000-0000-0000-0000-000000000000"],
      status: "done",
    });
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.updated).toBe(1);
    expect(body.notFound).toHaveLength(1);
  });
});
