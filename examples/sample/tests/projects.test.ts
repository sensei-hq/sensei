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

describe("GET /projects", () => {
  it("returns empty list initially", async () => {
    const res = await req("GET", "/projects");
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.data).toEqual([]);
    expect(body.nextCursor).toBeNull();
  });

  it("returns created projects", async () => {
    await req("POST", "/projects", { name: "Alpha" });
    await req("POST", "/projects", { name: "Beta" });
    const res = await req("GET", "/projects");
    const body = await res.json();
    expect(body.data).toHaveLength(2);
  });
});

describe("POST /projects", () => {
  it("creates a project with required fields", async () => {
    const res = await req("POST", "/projects", { name: "My Project" });
    expect(res.status).toBe(201);
    const body = await res.json();
    expect(body.name).toBe("My Project");
    expect(body.status).toBe("active");
    expect(body.id).toBeTruthy();
  });

  it("creates a project with optional description", async () => {
    const res = await req("POST", "/projects", { name: "X", description: "A description" });
    expect(res.status).toBe(201);
    expect((await res.json()).description).toBe("A description");
  });

  it("rejects empty name", async () => {
    const res = await req("POST", "/projects", { name: "" });
    expect(res.status).toBe(400);
  });

  it("rejects missing name", async () => {
    const res = await req("POST", "/projects", {});
    expect(res.status).toBe(400);
  });
});

describe("GET /projects/:id", () => {
  it("returns the project", async () => {
    const created = await (await req("POST", "/projects", { name: "Foo" })).json();
    const res = await req("GET", `/projects/${created.id}`);
    expect(res.status).toBe(200);
    expect((await res.json()).id).toBe(created.id);
  });

  it("returns 404 for unknown id", async () => {
    const res = await req("GET", "/projects/00000000-0000-0000-0000-000000000000");
    expect(res.status).toBe(404);
  });
});

describe("PATCH /projects/:id", () => {
  it("updates name", async () => {
    const p = await (await req("POST", "/projects", { name: "Old" })).json();
    const res = await req("PATCH", `/projects/${p.id}`, { name: "New" });
    expect(res.status).toBe(200);
    expect((await res.json()).name).toBe("New");
  });

  it("archives a project", async () => {
    const p = await (await req("POST", "/projects", { name: "Archived" })).json();
    const res = await req("PATCH", `/projects/${p.id}`, { status: "archived" });
    expect((await res.json()).status).toBe("archived");
  });
});

describe("DELETE /projects/:id", () => {
  it("deletes the project", async () => {
    const p = await (await req("POST", "/projects", { name: "Gone" })).json();
    expect((await req("DELETE", `/projects/${p.id}`)).status).toBe(204);
    expect((await req("GET", `/projects/${p.id}`)).status).toBe(404);
  });
});

describe("GET /projects/:id/tasks", () => {
  it("returns tasks scoped to the project", async () => {
    const p = await (await req("POST", "/projects", { name: "P" })).json();
    await req("POST", "/tasks", { project_id: p.id, title: "T1" });
    await req("POST", "/tasks", { project_id: p.id, title: "T2" });
    const res = await req("GET", `/projects/${p.id}/tasks`);
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.data).toHaveLength(2);
  });
});
