## ADDED Requirements

### Requirement: Push readiness is public
The dōjō `GET /version` SHALL report `push.configured`: true only when `PUBLIC_VAPID_KEY`,
`VAPID_SUBJECT` and `VAPID_PRIVATE_KEY` are all set. It SHALL NOT expose any key material.

#### Scenario: VAPID fully configured
- **WHEN** all three VAPID values are set and `GET /version` is called without a session
- **THEN** the response includes `"push": {"configured": true}` and no key values

#### Scenario: A VAPID value is missing
- **WHEN** `VAPID_PRIVATE_KEY` is unset
- **THEN** `GET /version` reports `"push": {"configured": false}`

### Requirement: Each push send records its outcome
Every attempted send to a subscription SHALL record `last_push_at`, `last_push_status` (the push
service's HTTP status, or null on a thrown error) and `last_push_error` (≤600 chars) on that
`dojo.push_subscriptions` row. Recording SHALL NOT make a relay-triggered send fail its caller.

#### Scenario: Delivered
- **WHEN** a push to a subscription returns 201
- **THEN** that row has `last_push_status = 201`, a fresh `last_push_at` and no `last_push_error`

#### Scenario: Push service error
- **WHEN** a push returns 500
- **THEN** the row records status 500 and the error text, and the remaining subscriptions are still tried

### Requirement: A person can read their push status
`GET /v1/you/push/status` SHALL return `configured` and the caller's own subscriptions with
`platform`, `enabled`, `last_push_at` and `last_push_status`, never the endpoint or keys. A read
failure SHALL be an error response, never an empty list.

#### Scenario: Own subscriptions only
- **WHEN** a signed-in user with one web subscription calls the route
- **THEN** it returns that subscription's status and no other user's rows, and no `web_push` field

### Requirement: A person can send themselves a test push
`POST /v1/you/push/test` SHALL send a fixed zero-knowledge test notification to the caller's enabled
subscriptions, ignoring event opt-ins and quiet hours, and return `{attempted, delivered, disabled}`.
Without VAPID it SHALL answer 503 `push_not_configured`; with no enabled subscription, 409
`no_subscription`.

#### Scenario: Test push delivered
- **WHEN** a user with one enabled subscription and all event opt-ins off sends a test
- **THEN** one push is sent with title "sensei" and no run content, and the response is `{attempted: 1, delivered: 1, disabled: []}`

#### Scenario: Push not configured
- **WHEN** VAPID is unset and a user sends a test
- **THEN** the response is 503 with `error: push_not_configured`

### Requirement: The inbox shows push readiness and a test action
The inbox "Notify me" control SHALL offer "Send a test" when notifications are on and show the last
delivery time; when the dōjō has no public VAPID key it SHALL show "Notifications aren't set up on
this dōjō" instead of an enable button. A failed test SHALL render a failure notice with its remedy.

#### Scenario: Test from the inbox (local dōjō, Playwright)
- **WHEN** a signed-in user on the local dōjō with a stub push endpoint turns on Notify me and presses Send a test
- **THEN** the stub receives one encrypted request and the control shows "Delivered" with the time, without a page refresh

#### Scenario: Not configured
- **WHEN** the dōjō has no `PUBLIC_VAPID_KEY`
- **THEN** the inbox shows "Notifications aren't set up on this dōjō" and no enable button
