/**
 * Window capture with a verdict (#284) — the one choke point every
 * screenshot in both harnesses goes through.
 */
import { readFileSync, rmSync } from 'node:fs';
import { isUniformPng, PAGE_BAND } from './png-blank';

export type ShotVerdict = 'ok' | 'blank' | 'unpainted' | 'unavailable';

/** Quiet after the settle frames before the first capture (WKWebView paints
 *  the page area some time after the DOM settles; relay-conformance needed
 *  1200ms before its evidence captures stopped coming back unpainted). */
const SETTLE_MS = 1200;
/** Captures before a flat frame is reported rather than retried. */
const TRIES = 3;
/** Quiet between a flat capture and its recapture. */
const RETRY_GAP_MS = 500;

export interface ShotOptions {
  settleMs?: number;
  tries?: number;
  retryGapMs?: number;
}

/**
 * Capture the app window to `path`, and say so when the capture is not a
 * picture of the app (#284). Two ways it fails:
 *
 * - `blank`: one flat colour. The window capture needs macOS Screen Recording
 *   permission for Sensei.app, and a rebuilt, re-signed bundle loses it (the
 *   e2e build is signed with a stable identity by `make app-e2e-build`; with
 *   the `sensei-dev` certificate the grant then survives rebuilds).
 * - `unpainted`: the frame and docked devtools paint, the page area does not —
 *   the capture landed while the compositor had no fresh webview surface.
 * - `unavailable`: the capture itself failed — no window on screen (a locked or
 *   sleeping display).
 *
 * The paint is settled for before the first capture, and a flat frame is
 * recaptured rather than reported at once: a capture taken the moment the DOM
 * settles is often unpainted, while the next one, a breath later, is fine.
 * Either way a flat file is removed — a blank PNG left behind reads as
 * evidence — and the cause is logged. The test does not fail on it: its DOM
 * assertions are the evidence; the picture is for a person.
 */
export async function shot(
  tauriPage: {
    screenshot: (o: { path: string }) => Promise<unknown>;
    evaluate: (script: string) => Promise<unknown>;
  },
  path: string,
  opts?: ShotOptions,
): Promise<ShotVerdict> {
  const settleMs = opts?.settleMs ?? SETTLE_MS;
  const tries = Math.max(1, opts?.tries ?? TRIES);
  const retryGapMs = opts?.retryGapMs ?? RETRY_GAP_MS;
  // Two animation frames, so at least one landed after whatever DOM work came
  // before — then quiet, so the compositor has it too. Best effort: a dead
  // webview fails at the capture below, with its own error. The sleep is not
  // skipped when the frames never come (a hidden page throttles rAF): it is
  // the settle that matters.
  try {
    await tauriPage.evaluate(
      `new Promise((r) => { setTimeout(r, 1000); requestAnimationFrame(() => requestAnimationFrame(r)); })`,
    );
  } catch {
    /* the capture below reports the breakage */
  }
  await new Promise((r) => setTimeout(r, settleMs));
  let result: ShotVerdict = 'unpainted';
  for (let attempt = 1; attempt <= tries; attempt++) {
    if (attempt > 1) await new Promise((r) => setTimeout(r, retryGapMs));
    try {
      await tauriPage.screenshot({ path });
    } catch (e) {
      // No window on screen to capture (a locked or sleeping display, #425): a
      // recapture a breath later finds none either, so it is reported at once.
      console.warn(`[shot] ${path}: no capture — ${e instanceof Error ? e.message : String(e)} (#284)`);
      return 'unavailable';
    }
    const png = readFileSync(path);
    result = isUniformPng(png) ? 'blank' : isUniformPng(png, PAGE_BAND) ? 'unpainted' : 'ok';
    if (result === 'ok') return 'ok';
    rmSync(path);
    const cause =
      result === 'blank'
        ? 'the capture is blank — grant Sensei.app Screen Recording in System Settings › Privacy & Security'
        : 'the page area did not paint in the capture';
    console.warn(
      attempt < tries
        ? `[shot] ${path}: ${cause} — recapturing (attempt ${attempt}/${tries}) (#284)`
        : `[shot] ${path}: ${cause} (#284)`,
    );
  }
  return result;
}
