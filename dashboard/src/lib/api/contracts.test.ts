import { describe, expect, it } from 'vitest';
import {
	mapAgentEventToAudit,
	mapAgentEventToIncident,
	normalizeAgentEventsPage,
	normalizeBlueprintList,
	normalizeBlueprintRunResult,
	normalizeConfigPayload,
	normalizeCrashSummary,
	normalizeRunAsyncResult,
	normalizePluginHealthMap
} from './contracts';

describe('API contracts normalization', () => {
	it('normalizes blueprint list from mixed payload shape', () => {
		const rows = normalizeBlueprintList({
			blueprints: [
				{ id: 'A', label: 'Kernel A', cat: 'kernel', desc: 'desc-a' },
				{ id: 'B', name: 'Net B', category: 'network', description: 'desc-b' },
				{ id: 'C', name: 'Bad', category: 'unsupported' }
			]
		});

		expect(rows).toHaveLength(3);
		expect(rows[0]).toMatchObject({
			id: 'A',
			name: 'Kernel A',
			category: 'kernel',
			description: 'desc-a'
		});
		expect(rows[1].category).toBe('network');
		expect(rows[2].category).toBe('kernel');
	});

	it('normalizes blueprint run result shape', () => {
		const a = normalizeBlueprintRunResult({ queued: true, jobId: 'job-1' });
		const b = normalizeBlueprintRunResult({ ok: 1 });

		expect(a).toEqual({ queued: true, jobId: 'job-1' });
		expect(b).toEqual({ queued: true, jobId: undefined });
	});

	it('normalizes plugin health map entries', () => {
		const map = normalizePluginHealthMap({
			scheduler: { status: 'OK', uptime: 120 },
			mesh: { status: 'degraded', error: 'timeout' }
		});

		expect(map.scheduler.status).toBe('ok');
		expect(map.scheduler.uptime).toBe(120);
		expect(map.mesh.error).toBe('timeout');
	});

	it('normalizes plugin list payload shape from agent plugins endpoint', () => {
		const map = normalizePluginHealthMap({
			plugins: [
				{ name: 'secureboot', status: 'DISCOVERED', runtime: 'powershell' },
				{ name: 'diagnostics', status: 'degraded', error: 'timeout' }
			]
		});

		expect(map.secureboot.status).toBe('discovered');
		expect(map.diagnostics.error).toBe('timeout');
	});

	it('normalizes events page with cursor metadata', () => {
		const page = normalizeAgentEventsPage({
			events: [
				{ id: 'e1', kind: 'job', ts_utc: '2026-01-01T00:00:00Z', action: 'doctor' },
				{ id: 'e2', kind: 'host', ts_utc: '2026-01-01T00:00:02Z', source: 'mesh' }
			],
			next_cursor: 'e1',
			returned: 2
		});

		expect(page.returned).toBe(2);
		expect(page.nextCursor).toBe('e1');
		expect(page.rows[0]).toMatchObject({ id: 'e1', kind: 'job', action: 'doctor' });
	});

	it('maps agent events to incident and audit records', () => {
		const page = normalizeAgentEventsPage([
			{ id: 'e100', kind: 'job', ts_utc: '2026-01-01T00:00:00Z', action: 'build_iso', status: 'failed', source: 'local' }
		]);
		const event = page.rows[0];

		const incident = mapAgentEventToIncident(event);
		expect(incident.id).toBe('e100');
		expect(incident.severity).toBe('high');

		const audit = mapAgentEventToAudit(event);
		expect(audit.id).toBe('e100');
		expect(audit.status).toBe('failure');
		expect(audit.operator).toBe('local');
	});

	it('normalizes run_async response when agent returns top-level id/action', () => {
		const result = normalizeRunAsyncResult({
			ok: true,
			id: 'job-123',
			action: 'doctor',
			priority: 'normal'
		});

		expect(result.accepted).toBe(true);
		expect(result.job?.id).toBe('job-123');
		expect(result.job?.action).toBe('doctor');
	});

	it('normalizes config payload when agent returns raw config.agent shape', () => {
		const payload = normalizeConfigPayload({
			config: {
				agent: {
					port: 7401,
					auth_mode: 'strict',
					max_queue: 100
				}
			}
		});

		expect(payload.values.port).toBe(7401);
		expect(payload.values.auth_mode).toBe('strict');
		expect(payload.values.max_queue).toBe(100);
	});

	it('normalizes crash summary from crash_count style response', () => {
		const crash = normalizeCrashSummary({
			crash_count: 2,
			artifacts_dir: 'artifacts/crash'
		});

		expect(crash.entries).toHaveLength(1);
		expect(crash.entries[0].path).toBe('artifacts/crash');
		expect(crash.entries[0].exists).toBe(true);
	});
});
