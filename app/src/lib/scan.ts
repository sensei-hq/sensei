// The app's one scan state (#496): the package's ScanState over the daemon's
// scan source. Every progress surface — the setup scan page, the sidebar
// minis, the footer band — imports this instance.
//
// Leaf imports only: this module is imported by wizard-state, which appstate
// imports, so reaching back into appstate (or api.js, which specs mock)
// would cycle. The port is a build-time constant; daemon-port.js is its home.
import { ScanState, scanClient, scanEvents, scanSseSource } from '@sensei/state';
import { daemonBaseUrl } from './daemon-port.js';

const base = daemonBaseUrl();

export const scanState = new ScanState(
  scanSseSource(scanClient(base), () => scanEvents(base)),
);
