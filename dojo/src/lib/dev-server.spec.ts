import { describe, expect, it } from 'vitest';
import { devServer } from '../../dev-server';

// #185: `vite dev` bound `localhost` as IPv6 only (`::1`), while the daemon's HTTP
// client resolves `localhost` to IPv4 — so `dojo_sync` against a local dōjō failed
// every tick with "could not reach dōjō", and nothing said why. The dev server
// binds the address the daemon dials, on the port its membership names.
describe('dojo dev server', () => {
	it('listens on IPv4 127.0.0.1:5173, and fails rather than moving port', () => {
		expect(devServer.host).toBe('127.0.0.1');
		expect(devServer.port).toBe(5173);
		expect(devServer.strictPort).toBe(true);
	});
});
