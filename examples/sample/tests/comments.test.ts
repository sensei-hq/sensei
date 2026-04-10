/**
 * Tests for the comments feature (feature1.md).
 * All tests here should FAIL until feature1 is implemented.
 */
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

async function makeProject() {
  return (await req("POST", "/projects", { name: "P" })).json();
}

async function makeTask(projectId: string) {
  return (await req("POST", "/tasks", { project_id: projectId, title: "T" })).json();
}

describe("GET /tasks/:id/comments — list comments (feature 1)", () => {
  it("returns empty list when no comments", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    const res = await req("GET", `/tasks/${t.id}/comments`);
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.data).toEqual([]);
    expect(body.nextCursor).toBeNull();
  });

  it("returns comments in descending order", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    await req("POST", `/tasks/${t.id}/comments`, { author: "alice", body: "First" });
    await req("POST", `/tasks/${t.id}/comments`, { author: "bob", body: "Second" });
    const res = await req("GET", `/tasks/${t.id}/comments`);
    const body = await res.json();
    expect(body.data).toHaveLength(2);
    expect(body.data[0].body).toBe("Second");
  });

  it("returns 404 for unknown task", async () => {
    const res = await req("GET", "/tasks/00000000-0000-0000-0000-000000000000/comments");
    expect(res.status).toBe(404);
  });
});

describe("POST /tasks/:id/comments — create comment (feature 1)", () => {
  it("creates a comment", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    const res = await req("POST", `/tasks/${t.id}/comments`, {
      author: "alice",
      body: "Looks good!",
    });
    expect(res.status).toBe(201);
    const comment = await res.json();
    expect(comment.author).toBe("alice");
    expect(comment.body).toBe("Looks good!");
    expect(comment.task_id).toBe(t.id);
  });

  it("rejects empty author", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    const res = await req("POST", `/tasks/${t.id}/comments`, { author: "", body: "Hi" });
    expect(res.status).toBe(400);
  });

  it("rejects empty body", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    const res = await req("POST", `/tasks/${t.id}/comments`, { author: "alice", body: "" });
    expect(res.status).toBe(400);
  });

  it("returns 404 for unknown task", async () => {
    const res = await req("POST", "/tasks/00000000-0000-0000-0000-000000000000/comments", {
      author: "alice",
      body: "Hi",
    });
    expect(res.status).toBe(404);
  });
});

describe("DELETE /comments/:id (feature 1)", () => {
  it("deletes the comment", async () => {
    const p = await makeProject();
    const t = await makeTask(p.id);
    const comment = await (
      await req("POST", `/tasks/${t.id}/comments`, { author: "alice", body: "To delete" })
    ).json();
    expect((await req("DELETE", `/comments/${comment.id}`)).status).toBe(204);
  });

  it("returns 404 for unknown comment", async () => {
    expect(
      (await req("DELETE", "/comments/00000000-0000-0000-0000-000000000000")).status,
    ).toBe(404);
  });
});
