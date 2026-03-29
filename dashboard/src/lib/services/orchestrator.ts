import { AgentRepo, LauncherRepo } from '$lib/api';
import { appState } from '$lib/state.svelte';
import { liveStream } from '$lib/services/live-stream';
import type { AuditStreamEvent, DashboardStreamEvent, IncidentStreamEvent } from '$lib/types';

class Orchestrator {
	private timer: ReturnType<typeof setInterval> | null = null;
	private syncInFlight = false;
	private currentIntervalMs = 0;
	private queuedAudits: AuditStreamEvent[] = [];
	private queuedIncidents: IncidentStreamEvent[] = [];
	private flushTimer: ReturnType<typeof setTimeout> | null = null;

	public async initialize() {
		appState.initializeSettings();
		if (this.timer) return;

		liveStream.start({
			onAudit: (payload) => this.ingestStreamAudit(payload),
			onIncident: (payload) => this.ingestStreamIncident(payload)
		});

		await this.sync(true);
		this.startPolling();
	}

	private startPolling(): void {
		if (this.timer) {
			clearInterval(this.timer);
			this.timer = null;
		}
		this.currentIntervalMs = appState.syncIntervalMs;
		this.timer = setInterval(() => {
			// Automatically pick up interval changes from settings.
			if (appState.syncIntervalMs !== this.currentIntervalMs) {
				this.startPolling();
				return;
			}
			void this.sync();
		}, this.currentIntervalMs);
	}

	public async sync(force = false) {
		if (this.syncInFlight && !force) {
			return;
		}

		this.syncInFlight = true;
		const start = Date.now();
		try {
			const [health, metrics, incidents, launcherStatus, pluginHealth] = await Promise.all([
				AgentRepo.fetchHealth(),
				AgentRepo.fetchMetrics(),
				AgentRepo.fetchIncidents(),
				LauncherRepo.fetchAgentStatus().catch(() => appState.launcherStatus),
				AgentRepo.fetchPluginHealth().catch(() => appState.pluginHealth)
			]);

			appState.isConnected = true;
			appState.latencyMs = Date.now() - start;
			appState.agentHealth = health;
			appState.launcherStatus = launcherStatus;
			appState.pluginHealth = pluginHealth;
			appState.metrics = { ...metrics, latency: appState.latencyMs };
			appState.incidents = incidents;
			appState.lastSyncAt = new Date().toISOString();
			appState.lastSyncError = null;

			try {
				const audit = await LauncherRepo.fetchAudit(40);
				if (audit.length > 0) {
					appState.auditLogs = audit;
				}
			} catch {
				// Launcher API may be unavailable; this should not mark agent as disconnected.
			}
		} catch (error) {
			appState.isConnected = false;
			appState.lastSyncError = error instanceof Error ? error.message : String(error);
		} finally {
			this.syncInFlight = false;
		}
	}

	public dispose() {
		if (this.timer) clearInterval(this.timer);
		this.timer = null;
		if (this.flushTimer) clearTimeout(this.flushTimer);
		this.flushTimer = null;
		this.queuedAudits = [];
		this.queuedIncidents = [];
		liveStream.stop();
	}

	private scheduleFlush(): void {
		if (this.flushTimer) return;
		this.flushTimer = setTimeout(() => {
			this.flushTimer = null;

			if (this.queuedAudits.length > 0) {
				const payload = [...this.queuedAudits];
				this.queuedAudits = [];
				const rows = payload.map((event) => event.payload);
				appState.auditLogs = [...rows.reverse(), ...appState.auditLogs].slice(0, 500);
			}

			if (this.queuedIncidents.length > 0) {
				const payload = [...this.queuedIncidents];
				this.queuedIncidents = [];
				const existing = new Map(appState.incidents.map((item) => [item.id, item]));
				for (const event of payload) {
					existing.set(event.payload.id, event.payload);
				}
				appState.incidents = Array.from(existing.values()).slice(0, 500);
			}
		}, 120);
	}

	private ingestStreamAudit(payload: unknown): void {
		const event = payload as DashboardStreamEvent;
		if (event.type !== 'audit') {
			return;
		}
		this.queuedAudits.push(event);
		this.scheduleFlush();
	}

	private ingestStreamIncident(payload: unknown): void {
		const event = payload as DashboardStreamEvent;

		if (event.type === 'metrics') {
			appState.metrics = { ...event.payload, latency: appState.latencyMs };
			return;
		}

		if (event.type === 'status') {
			appState.launcherStatus = event.payload;
			return;
		}

		if (event.type !== 'incident') {
			return;
		}
		this.queuedIncidents.push(event);
		this.scheduleFlush();
	}
}

export const orchestrator = new Orchestrator();
