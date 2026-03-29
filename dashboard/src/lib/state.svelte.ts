import {
	DEFAULT_AGENT_URL,
	DEFAULT_LOCAL_AGENT_TOKEN,
	DEFAULT_SYNC_INTERVAL_MS,
	STORAGE_KEY,
	clampSyncInterval,
	isLocaleCode,
	isThemeMode,
	normalizeAgentUrl
} from '$lib/config/dashboard-settings';
import { migrateSettingsProfile, serializeSettingsProfile } from '$lib/settings/profile';
import type {
	AgentHealth,
	AuditLog,
	DashboardSettingsDraft,
	Incident,
	LauncherStatus,
	LocaleCode,
	NodeMetrics,
	PluginHealthMap,
	SyncStatus,
	ThemeMode
} from './types';

class DashboardState {
	private hydrated = false;

	private applyLocalTokenFallback(): void {
		if (this.agentUrl !== DEFAULT_AGENT_URL) {
			return;
		}
		if (!this.agentToken || this.agentToken === 'change-me-hypercore-agent-token') {
			this.agentToken = DEFAULT_LOCAL_AGENT_TOKEN;
		}
	}

	public agentUrl = $state(DEFAULT_AGENT_URL);
	public agentToken = $state('');
	public launcherToken = $state('');
	public isConnected = $state(false);
	public launcherConnected = $state(false);
	public latencyMs = $state(0);
	public syncIntervalMs = $state(DEFAULT_SYNC_INTERVAL_MS);
	public streamConnected = $state(false);
	public liveMode = $state<'polling' | 'streaming'>('polling');
	public apiCircuitOpen = $state(false);
	public retryCount = $state(0);
	public lastSyncAt = $state<string | null>(null);
	public lastSyncError = $state<string | null>(null);
	public agentHealth = $state<AgentHealth>({ status: 'unknown' });
	public launcherStatus = $state<LauncherStatus>({ status: 'unknown', pid: null });
	public pluginHealth = $state<PluginHealthMap>({});

	public theme = $state<ThemeMode>('dark');
	public lang = $state<LocaleCode>('en');

	public metrics = $state<NodeMetrics>({
		cpu: 0,
		memory: 0,
		disk: 0,
		uptime: 0,
		latency: 0
	});
	public incidents = $state<Incident[]>([]);
	public auditLogs = $state<AuditLog[]>([]);

	public syncStatus = $derived<SyncStatus>(
		this.isConnected ? (this.metrics.cpu > 90 ? 'degraded' : 'online') : 'offline'
	);

	public connectionQuality = $derived.by<'stable' | 'recovering' | 'degraded' | 'offline'>(() => {
		if (!this.isConnected) return 'offline';
		if (this.apiCircuitOpen) return 'degraded';
		if (this.retryCount > 0) return 'recovering';
		if (this.latencyMs > 1500) return 'degraded';
		return 'stable';
	});

	public criticalIncidentCount = $derived(
		this.incidents.filter((incident) => incident.severity === 'critical').length
	);

	private applyDraft(draft: Partial<DashboardSettingsDraft>) {
		if (typeof draft.agentUrl === 'string') {
			this.agentUrl = normalizeAgentUrl(draft.agentUrl);
		}
		if (typeof draft.agentToken === 'string') {
			this.agentToken = draft.agentToken.trim();
		}
		if (typeof draft.launcherToken === 'string') {
			this.launcherToken = draft.launcherToken.trim();
		}
		if (typeof draft.syncIntervalMs === 'number') {
			this.syncIntervalMs = clampSyncInterval(draft.syncIntervalMs);
		}
		if (isThemeMode(draft.theme)) {
			this.theme = draft.theme;
		}
		if (isLocaleCode(draft.lang)) {
			this.lang = draft.lang;
		}
	}

	public addAudit(action: string, status: 'success' | 'failure' = 'success') {
		const entry: AuditLog = {
			id: crypto.randomUUID(),
			timestamp: new Date().toISOString(),
			action,
			operator: 'ROOT',
			status
		};
		this.auditLogs = [entry, ...this.auditLogs].slice(0, 500);
	}

	public initializeSettings() {
		if (this.hydrated || typeof window === 'undefined') {
			return;
		}

		this.hydrated = true;

		try {
			const raw = window.localStorage.getItem(STORAGE_KEY);
			if (!raw) {
				this.applyLocalTokenFallback();
				return;
			}

			const profile = migrateSettingsProfile(JSON.parse(raw));
			this.applyDraft(profile);
			this.applyLocalTokenFallback();
		} catch {
			// Corrupt settings are ignored to keep the dashboard bootable.
			this.applyLocalTokenFallback();
		}
	}

	public applyConnectionSettings(input: Partial<DashboardSettingsDraft>) {
		this.applyDraft(input);
		this.persistSettings();
	}

	public persistSettings() {
		if (typeof window === 'undefined') {
			return;
		}

		window.localStorage.setItem(
			STORAGE_KEY,
			JSON.stringify(
				serializeSettingsProfile({
					agentUrl: this.agentUrl,
					agentToken: this.agentToken,
					launcherToken: this.launcherToken,
					syncIntervalMs: this.syncIntervalMs,
					theme: this.theme,
					lang: this.lang
				})
			)
		);
	}
}

export const appState = new DashboardState();
