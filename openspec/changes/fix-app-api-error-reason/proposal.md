# Change: The app's API client shows the daemon's reason, not its error code (#451)

## Why
`app/src/lib/api.ts` keeps the response's `error` field as the message. Daemon routes that answer
`{ error: <code>, message: <reason> }` therefore surface a code ("relay_conflict") to the person.
The relay client had the same bug and was fixed in #426.

## What Changes
- Survey which daemon routes answer `{error, message}` and which answer `{error: <reason>}`.
- The general client prefers `message`, then `error`, then the status text — one shared helper with
  the relay client (`@sensei/state`), not two copies.

## Impact
- Specs: `app-api-client`. Code: `app/src/lib/api.ts`, `packages/state/relay/client.ts` (shared helper).
