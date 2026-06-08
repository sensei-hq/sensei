set search_path to sensei, extensions;

create type folder_role
    as enum (
        -- architectural component roles
        'backend', 'frontend', 'library', 'docs', 'infra',
        -- product / deployment / structure roles (from README frontmatter)
        'website', 'desktop', 'mobile', 'config', 'packaging'
    );
