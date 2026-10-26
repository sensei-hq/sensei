set search_path to sensei, extensions;

-- A new repository may own a transcript that ingest IGNORED because none of its
-- cwds was a tracked folder (#125). Clearing those marks makes the next ingest
-- pass read each such transcript again and attribute what it now can — which is
-- what widening the watch roots must do. A trigger, because repositories are
-- inserted on several code paths and every one of them must re-open the skips.
-- Subfolders are not repositories: they only appear inside a tracked one, so
-- they make nothing new attributable.
create or replace function forget_untracked_transcripts()
returns trigger
language plpgsql
as
$$
begin
  delete from activity.capture_watermarks where untracked;
  return new;
end;
$$;

drop trigger if exists folders_forget_untracked_transcripts on sensei.folders;
create trigger folders_forget_untracked_transcripts
after insert
    on sensei.folders
   for each row
  when (new.kind in ('git', 'standalone', 'subtree'))
       execute function forget_untracked_transcripts();
