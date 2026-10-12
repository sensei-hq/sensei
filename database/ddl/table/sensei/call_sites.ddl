set search_path to sensei, extensions;

create table if not exists call_sites (
  id           bigserial primary key
, folder_id    uuid      not null references sensei.folders(id) on delete cascade
, file_path    text      not null
, from_node_id uuid      not null references sensei.nodes(id) on delete cascade
, at           jsonb     not null
, callee       text      not null
, type_args    text[]    not null default '{}'
, receiver     jsonb     not null
, args         jsonb     not null
);

create index if not exists call_sites_folder_file_idx
    on call_sites(folder_id, file_path);

create index if not exists call_sites_folder_callee_idx
    on call_sites(folder_id, callee);

comment on table call_sites is
'What a call site was handed beyond its callee — string literals, lambdas, type
arguments and the receiver — for the calls that carry any of them (spec 20 §7b,
#251). A fact about the SOURCE: the walk records `.ToTable("orders")` without
knowing it is EF Core''s; the EF Core derivation reads these rows. Keyed by the
file that states them and REPLACED per file on re-index — the fact belongs to the
file, unlike a node.';

comment on column call_sites.from_node_id
     is 'The declaration the call sits in.';
comment on column call_sites.at
     is 'The invocation''s span, [start_line, start_col, end_line, end_col] — the same span its `calls` occurrence carries.';
comment on column call_sites.callee
     is 'The method name as written, without type arguments.';
comment on column call_sites.type_args
     is 'Type arguments verbatim: `Entity<Order>()` → {Order}.';
comment on column call_sites.receiver
     is '{"kind":"none"} | {"kind":"call","at":[…]} | {"kind":"name","name":…} | {"kind":"other"}.';
comment on column call_sites.args
     is 'Positional: {"str":…} | {"lambda":{"params":[…],"member":…|null,"at":[…]}} | {"other":true}.';
