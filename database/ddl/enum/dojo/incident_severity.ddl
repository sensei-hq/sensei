set search_path to dojo, extensions;

-- Severity of a confidentiality incident, declared in ascending order so
-- `ORDER BY severity DESC` surfaces the worst first. Drives SLA and alerting
-- on the client-lead console.
create type dojo.incident_severity
    as enum ('low', 'medium', 'high', 'critical');
