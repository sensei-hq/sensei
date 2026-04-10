/**
 * Date utilities for task deadline management.
 * All functions treat dates as UTC internally.
 */

export function formatDueDate(date: Date): string {
  const now = new Date();
  const diffMs = date.getTime() - now.getTime();
  const diffDays = Math.ceil(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays < 0) return `overdue by ${Math.abs(diffDays)}d`;
  if (diffDays === 0) return "due today";
  if (diffDays === 1) return "due tomorrow";
  if (diffDays <= 7) return `due in ${diffDays}d`;

  return date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: date.getFullYear() !== now.getFullYear() ? "numeric" : undefined,
  });
}

export function isPastDue(dueDate: Date, now = new Date()): boolean {
  return dueDate.getTime() < now.getTime();
}

export function isToday(date: Date, now = new Date()): boolean {
  return (
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  );
}

/** Returns true if the due date falls within the next `hours` hours. */
export function isDueSoon(dueDate: Date, hours: number, now = new Date()): boolean {
  const windowMs = hours * 60 * 60 * 1000;
  const diff = dueDate.getTime() - now.getTime();
  return diff > 0 && diff <= windowMs;
}

/** Start of day (00:00:00.000) for the given date. */
export function startOfDay(date: Date): Date {
  const d = new Date(date);
  d.setHours(0, 0, 0, 0);
  return d;
}

/** End of day (23:59:59.999) for the given date. */
export function endOfDay(date: Date): Date {
  const d = new Date(date);
  d.setHours(23, 59, 59, 999);
  return d;
}
