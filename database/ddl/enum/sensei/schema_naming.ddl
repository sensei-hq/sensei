set search_path to sensei, extensions;

-- Whether an object's NAME is what its source wrote (`stated`: DDL,
-- `.ToTable("orders")`) or what a convention supplied (`convention`: a
-- `DbSet<Order> Orders`, an ActiveRecord class name). Never promoted to
-- `stated` by agreement with another producer.
create type schema_naming
    as enum ('stated', 'convention');
