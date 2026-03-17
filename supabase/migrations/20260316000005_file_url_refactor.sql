-- supabase/migrations/20260316000005_file_url_refactor.sql
-- Migrate local_path → file:// base_url, then drop local_path columns.
-- documents_in_library.local_path is intentionally kept (document-level output).

-- 1. Populate base_url from local_path for libraries (guard: column may already be dropped)
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'sensei' AND table_name = 'libraries' AND column_name = 'local_path') THEN
    UPDATE sensei.libraries SET base_url = 'file://' || local_path WHERE local_path IS NOT NULL AND (base_url IS NULL OR base_url = '');
  END IF;
END;
$$;

-- 2. Populate base_url from local_path for referenced_libraries (guard: column may already be dropped)
DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = 'sensei' AND table_name = 'referenced_libraries' AND column_name = 'local_path') THEN
    UPDATE sensei.referenced_libraries SET base_url = 'file://' || local_path WHERE local_path IS NOT NULL AND (base_url IS NULL OR base_url = '');
  END IF;
END;
$$;

-- 3. Drop local_path from libraries
ALTER TABLE sensei.libraries DROP COLUMN IF EXISTS local_path;

-- 4. Drop local_path from referenced_libraries
ALTER TABLE sensei.referenced_libraries DROP COLUMN IF EXISTS local_path;

-- 5. Make base_url NOT NULL (every library now has a URL)
ALTER TABLE sensei.libraries ALTER COLUMN base_url SET NOT NULL;
ALTER TABLE sensei.referenced_libraries ALTER COLUMN base_url SET NOT NULL;
