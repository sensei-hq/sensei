-- Seed data for sensei schema
-- Provides realistic development data for the web app dashboard

-- 1. Repos
INSERT INTO sensei.repos (name, remote_url, default_branch, description, stack)
VALUES
  ('sensei', 'https://github.com/example/sensei', 'main',
   'AI-powered codebase indexer and context manager',
   ARRAY['TypeScript', 'Bun', 'Node.js']),
  ('my-app', null, 'main',
   'A sample web application',
   ARRAY['TypeScript', 'SvelteKit', 'PostgreSQL'])
ON CONFLICT (remote_url) WHERE remote_url IS NOT NULL DO NOTHING;

-- 2. Libraries
INSERT INTO sensei.libraries (name, ecosystem, version, description)
VALUES
  ('svelte', 'npm', '5.0.0', 'Cybernetically enhanced web apps'),
  ('supabase-js', 'npm', '2.47.0', 'Supabase JavaScript client')
ON CONFLICT (ecosystem, name) DO NOTHING;

-- 3. Events (3 recent tool usage events)
INSERT INTO sensei.events (user_uuid, session_id, repo_id, phase, tool, ts)
SELECT
  'demo-user-001', 'session-abc123', r.id, 'post', 'load_context', now() - interval '45 minutes'
FROM sensei.repos r WHERE r.name = 'sensei';

INSERT INTO sensei.events (user_uuid, session_id, repo_id, phase, tool, ts)
SELECT
  'demo-user-001', 'session-abc123', r.id, 'post', 'get_llmspec', now() - interval '30 minutes'
FROM sensei.repos r WHERE r.name = 'sensei';

INSERT INTO sensei.events (user_uuid, session_id, repo_id, phase, tool, ts)
SELECT
  'demo-user-001', 'session-abc123', r.id, 'post', 'edit', now() - interval '10 minutes'
FROM sensei.repos r WHERE r.name = 'sensei';

-- 4. Benchmark report
INSERT INTO sensei.benchmark_reports (repo_id, run_name, strategy, score, tokens, elapsed_ms, promoted)
SELECT r.id, 'baseline-v1', 'bm25+semantic', 0.82, 4200, 1850, false
FROM sensei.repos r WHERE r.name = 'sensei';
