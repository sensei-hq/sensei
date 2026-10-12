set search_path to sensei, extensions;

-- WHO described a schema object. dbd is the schema describing itself; every
-- other value is an ORM or data-access layer reading it. Carried so a consumer
-- can tell the schema's own word from an ORM's reading of it — they disagree,
-- and the schema wins. `other` keeps the detected dependency name in the row's
-- note: an ORM nobody enumerated yet is still a real mapping.
create type schema_source
    as enum ('dbd', 'ef_core', 'dapper', 'sqlalchemy', 'django_orm', 'jpa',
             'prisma', 'drizzle', 'typeorm', 'active_record', 'gorm', 'other');
