## ADDED Requirements

### Requirement: Cross-class severity mapping

Every failure class SHALL carry a severity from one mapping that joins
reason codes, notify levels, schedule standing, and sync states.

#### Scenario: Severities compare across classes

- **WHEN** a task fails terminally, a schedule fails, and a sync entity
  errors in the same window
- **THEN** each alert carries a severity from the same scale, and the feed
  orders critical above warnings regardless of class

### Requirement: Dedupe with success-clear

Alerts SHALL dedupe by `(class, key, window)` and clear on success or
reconnect.

#### Scenario: A flapping job alerts once

- **WHEN** a job fails repeatedly in one window
- **THEN** it raises one alert (counted), and the next success clears it
  instead of leaving a stale alarm

### Requirement: Proactive notify beyond assistant-capture

The system MUST proactively notify terminal task failure, retry
exhaustion, run crash, failing schedules, and daemon degraded/down through
the existing notify channel.

#### Scenario: An overnight failure is waiting in the morning

- **WHEN** a scheduled pass fails terminally with no screen open
- **THEN** a notification names the job, the reason, and the way back to
  the center's Failures feed

### Requirement: Daemon-down and provisioning banners

Daemon-down and first-install provisioning SHALL render global banners,
closing the two states the banner ignores today.

#### Scenario: A dead daemon says so

- **WHEN** the daemon stops answering health
- **THEN** a global banner names the outage (not silence), distinct from
  the degradedDB and config-error banners

### Requirement: Fail-closed alert reads

Alert-feed reads SHALL fail closed: a read failure surfaces as an error,
never an empty "all clear".

#### Scenario: A broken feed never reads as healthy

- **WHEN** the failures feed cannot be read
- **THEN** the surface renders the read error instead of an empty list
