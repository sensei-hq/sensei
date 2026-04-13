-- Core schema for a task management system

CREATE TABLE projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    archived    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE tasks (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id),
    title       TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'open',
    priority    INTEGER NOT NULL DEFAULT 0,
    assignee    TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_tasks_project ON tasks(project_id);
CREATE INDEX idx_tasks_status ON tasks(status);

-- Full-text search for tasks
CREATE VIRTUAL TABLE tasks_fts USING fts5(title, content=tasks, content_rowid=rowid);

-- Trigger to keep FTS in sync
CREATE TRIGGER tasks_ai AFTER INSERT ON tasks BEGIN
    INSERT INTO tasks_fts(rowid, title) VALUES (new.rowid, new.title);
END;

-- View for active tasks with project info
CREATE VIEW active_tasks AS
SELECT t.*, p.name AS project_name
FROM tasks t
JOIN projects p ON t.project_id = p.id
WHERE t.status != 'done' AND p.archived = 0;

-- Function to count tasks by status
CREATE TRIGGER update_timestamp AFTER UPDATE ON tasks BEGIN
    UPDATE tasks SET updated_at = datetime('now') WHERE id = new.id;
END;
