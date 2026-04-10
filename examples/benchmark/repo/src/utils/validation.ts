import type { CreateTaskInput, UpdateTaskInput, Priority, TaskStatus } from "../types.js";

export class ValidationError extends Error {
  constructor(
    message: string,
    public readonly field: string,
  ) {
    super(message);
    this.name = "ValidationError";
  }
}

const VALID_STATUSES: TaskStatus[] = ["todo", "in-progress", "done", "cancelled"];
const VALID_PRIORITIES: Priority[] = [1, 2, 3];
const MAX_TITLE_LENGTH = 200;
const MAX_TAGS = 10;
const MAX_TAG_LENGTH = 50;

export function validateTitle(title: unknown): string {
  if (typeof title !== "string" || title.trim().length === 0) {
    throw new ValidationError("Title must be a non-empty string", "title");
  }
  if (title.trim().length > MAX_TITLE_LENGTH) {
    throw new ValidationError(`Title must be ${MAX_TITLE_LENGTH} characters or fewer`, "title");
  }
  return title.trim();
}

export function validatePriority(priority: unknown): Priority {
  if (!VALID_PRIORITIES.includes(priority as Priority)) {
    throw new ValidationError("Priority must be 1 (high), 2 (medium), or 3 (low)", "priority");
  }
  return priority as Priority;
}

export function validateStatus(status: unknown): TaskStatus {
  if (!VALID_STATUSES.includes(status as TaskStatus)) {
    throw new ValidationError(`Status must be one of: ${VALID_STATUSES.join(", ")}`, "status");
  }
  return status as TaskStatus;
}

export function validateTags(tags: unknown): string[] {
  if (!Array.isArray(tags)) {
    throw new ValidationError("Tags must be an array", "tags");
  }
  if (tags.length > MAX_TAGS) {
    throw new ValidationError(`Maximum ${MAX_TAGS} tags allowed`, "tags");
  }
  const cleaned = tags.map((t, i) => {
    if (typeof t !== "string" || t.trim().length === 0) {
      throw new ValidationError(`Tag at index ${i} must be a non-empty string`, "tags");
    }
    if (t.trim().length > MAX_TAG_LENGTH) {
      throw new ValidationError(`Tag "${t}" exceeds ${MAX_TAG_LENGTH} characters`, "tags");
    }
    return t.trim().toLowerCase();
  });
  const unique = [...new Set(cleaned)];
  if (unique.length !== cleaned.length) {
    throw new ValidationError("Tags must be unique", "tags");
  }
  return unique;
}

export function validateDueDate(dueDate: unknown): Date | undefined {
  if (dueDate === undefined || dueDate === null) return undefined;
  const d = new Date(dueDate as string);
  if (isNaN(d.getTime())) {
    throw new ValidationError("dueDate must be a valid ISO date string", "dueDate");
  }
  return d;
}

export function validateCreateInput(input: CreateTaskInput): Required<Omit<CreateTaskInput, "description" | "assignee" | "dueDate">> & Pick<CreateTaskInput, "description" | "assignee" | "dueDate"> {
  return {
    title: validateTitle(input.title),
    description: input.description,
    priority: input.priority !== undefined ? validatePriority(input.priority) : 2,
    tags: input.tags !== undefined ? validateTags(input.tags) : [],
    assignee: input.assignee,
    dueDate: input.dueDate,
  };
}

export function validateUpdateInput(input: UpdateTaskInput): UpdateTaskInput {
  const result: UpdateTaskInput = {};
  if (input.title !== undefined) result.title = validateTitle(input.title);
  if (input.priority !== undefined) result.priority = validatePriority(input.priority);
  if (input.status !== undefined) result.status = validateStatus(input.status);
  if (input.tags !== undefined) result.tags = validateTags(input.tags);
  if (input.dueDate !== undefined) result.dueDate = input.dueDate;
  if (input.description !== undefined) result.description = input.description;
  if (input.assignee !== undefined) result.assignee = input.assignee;
  return result;
}
