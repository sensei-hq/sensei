// Operations alert toasts: the layout starts this beside the identity
// attention wiring, and standing failures toast through rokkit's AlertList.
// The push is injected so the mapping is unit-testable without rokkit.
import { alertsState } from './ops.js';
import type { OpsAlert } from '@sensei/state';

export interface OpsToast {
  type: 'error' | 'warning';
  text: string;
  timeout: number;
}

export type AlertPush = (toast: OpsToast) => void;

/** One alert's toast: the title, the reason, and the way back to the center. */
export function opsAlertToast(alert: OpsAlert): OpsToast {
  return {
    type: alert.level === 'critical' ? 'error' : 'warning',
    text: `${alert.title} — ${alert.detail} (Operations)`,
    timeout: alert.level === 'critical' ? 15000 : 8000,
  };
}

/** Starts the alert poller; every new warning/critical toasts through `push`. */
export function startOpsAlerts(push: AlertPush): void {
  alertsState.start((alert) => push(opsAlertToast(alert)));
}

/** Stops the alert poller (window teardown). */
export function stopOpsAlerts(): void {
  alertsState.stop();
}
