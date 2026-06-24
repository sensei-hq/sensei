set search_path to sensei, extensions;

-- Quality dimension for a memory in the learnings UI anatomy, orthogonal to
-- `memory_type` (which is the structural kind). A memory can be type=pattern
-- AND category=correctness. Used to group/colour learnings, not for resolution.
create type memory_category
    as enum ('correctness', 'convention', 'pattern', 'preference');
