// The app's one ops state per domain: the package's jobs/schedules/sync
// states over the daemon's ops source. Every operations surface — the
// center, the alert wiring — imports these instances, so navigation never
// loses them (6.3).
//
// Leaf imports only: like scan.js, this module must not reach back into
// appstate (or api.js, which specs mock) — the port is a build-time
// constant; daemon-port.js is its home.
import {
  AlertsState,
  JobsState,
  SchedulesState,
  SyncState,
  jobsSseSource,
  opsClient,
  opsEvents,
  schedulesSseSource,
  syncSseSource,
} from '@sensei/state';
import { daemonBaseUrl } from './daemon-port.js';

const base = daemonBaseUrl();
const api = opsClient(base);

export const jobsState = new JobsState(jobsSseSource(api, () => opsEvents(base)));
export const schedulesState = new SchedulesState(schedulesSseSource(api, () => opsEvents(base)));
export const syncState = new SyncState(syncSseSource(api, () => opsEvents(base)));

/** The one alert store, derived from the three domain states above. */
export const alertsState = new AlertsState({ jobs: jobsState, schedules: schedulesState, sync: syncState });

/** The raw ops client for one-shot reads the states don't own (drill-down, retry, backfill). */
export const opsApi = api;
