set search_path to dojo, extensions;

-- How a person sees sensei's marks (#133): every glyph a kanji, or every glyph
-- an icon. One rule across the app and the dōjō; the default is kanji.
do $$ begin
  create type dojo.glyph_mode as enum ('kanji', 'icon');
exception when duplicate_object then null; end $$;

comment on type dojo.glyph_mode is
'How a person sees sensei''s marks (#133): kanji (every glyph a kanji, the default) or icon (every glyph an icon).';
