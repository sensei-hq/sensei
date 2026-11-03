set search_path to dojo, extensions;

create table if not exists dojo.user_preferences (
  user_id        uuid             primary key
, glyphs         dojo.glyph_mode  not null default 'kanji'
, glyphs_set_at  timestamptz
, updated_at     timestamptz      not null default now()
);

comment on table dojo.user_preferences is
'A person''s own display preferences, synced with the desktop app (#133). One row per
person; read and written only as that person (policies/dojo/user_preferences.sql).
The daemon syncs through the same route as its signed-in person: the later change
wins, by when it was made, not when it arrived.';
comment on column dojo.user_preferences.user_id       is 'The person (dojo.principals.id) — the row is theirs alone.';
comment on column dojo.user_preferences.glyphs        is 'Kanji or icons, for every mark in the app and the dōjō (#133). Default kanji.';
comment on column dojo.user_preferences.glyphs_set_at is 'When the person chose `glyphs`, wherever they chose it; the later choice wins a sync. NULL: never chosen, so any choice wins.';
comment on column dojo.user_preferences.updated_at    is 'When the row was last written.';
