set search_path to sensei, extensions;

create or replace view assistant_catalogue_words as
select c.id   as word
     , c.id
     , c.name
     , c.kind
     , c.joins_crew
     , c.launch_template
  from sensei.assistant_catalogue c
 union all
select a.word
     , c.id
     , c.name
     , c.kind
     , c.joins_crew
     , c.launch_template
  from sensei.assistant_catalogue c
 cross join lateral unnest(c.aliases) as a(word);

comment on view assistant_catalogue_words is
'Every spelling of a supported tool (#455): one row for its id and one per alias,
each naming the tool. Join any family, acp_id or host word here to name and
classify it; a word with no row is unknown.';
comment on column assistant_catalogue_words.word
     is 'A spelling the system writes (the id or an alias).';
