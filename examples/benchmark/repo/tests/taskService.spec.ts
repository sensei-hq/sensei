import { describe, it, expect, beforeEach } from "vitest";
import { TaskService } from "../src/services/taskService.js";
import { ValidationError } from "../src/utils/validation.js";

describe("TaskService", () => {
  let svc: TaskService;

  beforeEach(() => {
    svc = new TaskService();
  });

  describe("createTask", () => {
    it("creates a task with required fields only", () => {
      const task = svc.createTask({ title: "Fix bug" });
      expect(task.id).toBeTruthy();
      expect(task.title).toBe("Fix bug");
      expect(task.status).toBe("todo");
      expect(task.priority).toBe(2);
      expect(task.tags).toEqual([]);
    });

    it("normalises tags to lowercase and deduplicates", () => {
      const task = svc.createTask({ title: "T", tags: ["Alpha", "alpha", "BETA"] });
      expect(task.tags).toEqual(["alpha", "beta"]);
    });

    it("throws ValidationError for empty title", () => {
      expect(() => svc.createTask({ title: "" })).toThrow(ValidationError);
    });
  });

  describe("filterTasks", () => {
    beforeEach(() => {
      svc.createTask({ title: "A", priority: 1, tags: ["urgent"] });
      svc.createTask({ title: "B", priority: 2, tags: ["bug"] });
      svc.createTask({ title: "C", priority: 3 });
      const d = svc.createTask({ title: "D" });
      svc.updateTask(d.id, { assignee: "alice" });
    });

    it("filters by priority", () => {
      expect(svc.filterTasks({ priority: 1 })).toHaveLength(1);
    });

    it("filters by tag", () => {
      expect(svc.filterTasks({ tag: "urgent" })).toHaveLength(1);
    });

    it("filters by assignee", () => {
      expect(svc.filterTasks({ assignee: "alice" })).toHaveLength(1);
    });

    it("returns all tasks when no filter applied", () => {
      expect(svc.filterTasks({})).toHaveLength(4);
    });
  });

  describe("updateTask", () => {
    it("updates status", () => {
      const t = svc.createTask({ title: "T" });
      const updated = svc.updateTask(t.id, { status: "in-progress" });
      expect(updated.status).toBe("in-progress");
    });

    it("throws for unknown id", () => {
      expect(() => svc.updateTask("nope", { status: "done" })).toThrow("not found");
    });
  });
});
