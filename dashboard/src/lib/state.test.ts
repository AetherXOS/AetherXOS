import { describe, it, expect, beforeEach } from 'vitest';
import { appState } from './state.svelte';

/**
 * Dashboard state engine verification suite
 */
describe('Dashboard Global State', () => {
	beforeEach(() => {
		appState.isConnected = false;
		appState.auditLogs = [];
	});

	it('should initialize with default connection settings', () => {
		expect(appState.agentUrl).toContain('127.0.0.1');
		expect(appState.isConnected).toBe(false);
	});

	it('should correctly derive sync status from metrics', () => {
		appState.isConnected = true;
		appState.metrics.cpu = 95;
		expect(appState.syncStatus).toBe('degraded');

		appState.metrics.cpu = 40;
		expect(appState.syncStatus).toBe('online');
	});

	it('should record and cap audit logs correctly', () => {
		appState.addAudit('TEST_ACTION');
		expect(appState.auditLogs.length).toBe(1);
		expect(appState.auditLogs[0].action).toBe('TEST_ACTION');
	});
});
