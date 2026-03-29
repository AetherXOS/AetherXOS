import axios, {
	type AxiosInstance,
	type AxiosResponse,
	type InternalAxiosRequestConfig
} from 'axios';
import { API_RESILIENCE } from '$lib/config/runtime-client';
import {
	normalizeAgentHealth,
	normalizeAgentCatalog,
	normalizeAgentEventsPage,
	normalizeBuildFeatureRecommendation,
	normalizeComplianceReport,
	normalizeConfigMutationResult,
	normalizeConfigDriftReport,
	normalizeConfigOverrideTemplate,
	normalizeConfigPayload,
	normalizeConfigProfileExport,
	normalizeConfirmationTicket,
	normalizeCrashSummary,
	normalizeHostList,
	normalizeJobDetail,
	normalizeJobSummary,
	normalizeJobList,
	normalizeJobMutationResult,
	normalizeAuditLogs,
	normalizeBlueprintList,
	normalizeBlueprintRunResult,
	normalizeIncidents,
	normalizeLauncherStatus,
	normalizeMetrics,
	normalizePluginHealthMap,
	normalizeRunAsyncResult,
	mapAgentEventToIncident,
	mapAgentEventToAudit
} from '$lib/api/contracts';
import { appState } from '$lib/state.svelte';
import type {
	AgentHealth,
	AgentHost,
	AgentCatalogAction,
	AgentConfigMutationResult,
	AgentConfigPayload,
	AgentEventsPage,
	AgentJobDetail,
	AgentJobStreamEvent,
	AgentJobMutationResult,
	AgentRunAsyncResult,
	AgentJobSummary,
	AuditLog,
	Blueprint,
	BlueprintRunResult,
	BuildFeatureRecommendation,
	ComplianceReport,
	ConfirmationTicket,
	CrashSummary,
	ConfigDriftReport,
	ConfigOverrideTemplate,
	ConfigProfileExport,
	ConfigUpdateEntry,
	Incident,
	LauncherStatus,
	NodeMetrics,
	PluginHealthMap
} from '$lib/types';

const RETRYABLE_STATUSES = new Set(API_RESILIENCE.retryableStatuses);

export type ApiErrorKind =
	| 'circuit_open'
	| 'network'
	| 'timeout'
	| 'unauthorized'
	| 'forbidden'
	| 'not_found'
	| 'rate_limited'
	| 'server'
	| 'unknown';

export class ApiRequestError extends Error {
	public readonly kind: ApiErrorKind;
	public readonly status?: number;
	public readonly retryable: boolean;
	public readonly service: string;
	public readonly operation: string;
	public readonly cause?: unknown;

	constructor(input: {
		kind: ApiErrorKind;
		message: string;
		service: string;
		operation: string;
		status?: number;
		retryable: boolean;
		cause?: unknown;
	}) {
		super(input.message);
		this.name = 'ApiRequestError';
		this.kind = input.kind;
		this.status = input.status;
		this.retryable = input.retryable;
		this.service = input.service;
		this.operation = input.operation;
		this.cause = input.cause;
	}
}

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function toErrorMessage(value: unknown): string {
	return value instanceof Error ? value.message : String(value);
}

function classifyApiError(error: unknown): { kind: ApiErrorKind; status?: number; message: string } {
	if (!axios.isAxiosError(error)) {
		return { kind: 'unknown', message: toErrorMessage(error) };
	}

	if (error.code === 'ECONNABORTED') {
		return { kind: 'timeout', message: 'Request timed out' };
	}

	if (!error.response) {
		return { kind: 'network', message: 'Network connection failed' };
	}

	const status = error.response.status;
	if (status === 401) return { kind: 'unauthorized', status, message: 'Unauthorized' };
	if (status === 403) return { kind: 'forbidden', status, message: 'Forbidden' };
	if (status === 404) return { kind: 'not_found', status, message: 'Resource not found' };
	if (status === 429) return { kind: 'rate_limited', status, message: 'Rate limited' };
	if (status >= 500) return { kind: 'server', status, message: 'Server error' };

	return { kind: 'unknown', status, message: toErrorMessage(error) };
}

function isNetworkOrServerError(error: unknown): boolean {
	if (!axios.isAxiosError(error)) return false;
	if (!error.response) return true;
	return error.response.status >= 500;
}

function parseSseBlock(rawBlock: string): { event: string; data: string } | null {
	const lines = rawBlock.replace(/\r/g, '').split('\n');
	let event = 'message';
	const dataLines: string[] = [];
	for (const line of lines) {
		if (!line || line.startsWith(':')) continue;
		if (line.startsWith('event:')) {
			event = line.slice(6).trim() || 'message';
			continue;
		}
		if (line.startsWith('data:')) {
			dataLines.push(line.slice(5).trimStart());
		}
	}
	if (dataLines.length === 0) return null;
	return { event, data: dataLines.join('\n') };
}

function tryParseRecord(raw: string): Record<string, unknown> | null {
	try {
		const parsed = JSON.parse(raw);
		if (parsed && typeof parsed === 'object') return parsed as Record<string, unknown>;
		return null;
	} catch {
		return null;
	}
}

function toJobStreamEvent(
	eventName: string,
	payload: Record<string, unknown> | null
): AgentJobStreamEvent {
	const payloadType = typeof payload?.type === 'string' ? payload.type.toLowerCase() : '';
	const normalizedSource = eventName === 'message' && payloadType ? payloadType : eventName;
	const normalizedEvent =
		normalizedSource === 'hello' ||
		normalizedSource === 'tail' ||
		normalizedSource === 'heartbeat' ||
		normalizedSource === 'done' ||
		normalizedSource === 'timeout' ||
		normalizedSource === 'error' ||
		normalizedSource === 'snapshot' ||
		normalizedSource === 'line' ||
		normalizedSource === 'complete'
			? normalizedSource
			: 'heartbeat';
	const jobRaw = payload?.job;
	const job = jobRaw && typeof jobRaw === 'object' ? normalizeJobSummary(jobRaw) : undefined;
	const detail = normalizedEvent === 'tail' ? normalizeJobDetail(payload ?? {}) : undefined;
	const lineCount =
		typeof payload?.line_count === 'number'
			? payload.line_count
			: typeof payload?.index === 'number'
				? payload.index + 1
			: (detail?.lineCount ?? job?.line_count);
	const message =
		typeof payload?.line === 'string'
			? payload.line
			: typeof payload?.message === 'string'
				? payload.message
				: undefined;

	return {
		type: normalizedEvent,
		detail,
		job,
		lineCount,
		message
	};
}

function shouldRetry(error: unknown): boolean {
	if (!axios.isAxiosError(error)) return false;
	if (!error.response) return true;
	return RETRYABLE_STATUSES.has(error.response.status);
}

interface ApiClientOptions {
	baseUrlGetter?: () => string;
	tokenHeader?: string;
	tokenGetter?: () => string;
	onDisconnect?: () => void;
	onCircuitChange?: (open: boolean) => void;
	serviceLabel?: string;
	timeoutMs?: number;
}

class DashboardApiClient {
	private readonly client: AxiosInstance;
	private readonly opts: ApiClientOptions;
	private consecutiveFailures = 0;
	private circuitOpenUntil = 0;

	constructor(opts: ApiClientOptions = {}) {
		this.opts = opts;
		this.client = axios.create({ timeout: opts.timeoutMs ?? API_RESILIENCE.defaultTimeoutMs });

		this.client.interceptors.request.use((config: InternalAxiosRequestConfig) => {
			const baseUrl = opts.baseUrlGetter?.();
			if (baseUrl) config.baseURL = baseUrl;

			const token = opts.tokenGetter?.();
			if (token && opts.tokenHeader) {
				config.headers[opts.tokenHeader] = token;
			}

			return config;
		});

		this.client.interceptors.response.use(
			(res: AxiosResponse) => res,
			(err: unknown) => {
				if (isNetworkOrServerError(err)) {
					opts.onDisconnect?.();
				}
				throw err;
			}
		);
	}

	private get isCircuitOpen(): boolean {
		return Date.now() < this.circuitOpenUntil;
	}

	private markSuccess(): void {
		const wasInFailureState = this.consecutiveFailures > 0 || this.circuitOpenUntil > 0;
		this.consecutiveFailures = 0;
		this.circuitOpenUntil = 0;
		appState.retryCount = 0;
		if (wasInFailureState) {
			this.opts.onCircuitChange?.(false);
		}
	}

	private markFailure(message: string): void {
		this.consecutiveFailures += 1;
		if (this.consecutiveFailures >= API_RESILIENCE.circuitOpenAfterFailures) {
			this.circuitOpenUntil = Date.now() + API_RESILIENCE.circuitOpenMs;
			this.opts.onCircuitChange?.(true);
			appState.addAudit(
				`API_CIRCUIT_OPEN [${this.opts.serviceLabel ?? 'API'}]: ${message}`,
				'failure'
			);
		}
	}

	private async executeWithResilience<T>(request: () => Promise<AxiosResponse<T>>): Promise<T> {
		if (this.isCircuitOpen) {
			throw new ApiRequestError({
				kind: 'circuit_open',
				message: `[${this.opts.serviceLabel ?? 'API'}] Circuit breaker is open. Waiting for cooldown.`,
				service: this.opts.serviceLabel ?? 'API',
				operation: 'request',
				retryable: false
			});
		}

		let attempt = 0;
		while (attempt <= API_RESILIENCE.maxRetryAttempts) {
			try {
				const response = await request();
				this.markSuccess();
				return response.data;
			} catch (error) {
				const classified = classifyApiError(error);
				if (attempt >= API_RESILIENCE.maxRetryAttempts || !shouldRetry(error)) {
					this.markFailure(toErrorMessage(error));
					throw new ApiRequestError({
						kind: classified.kind,
						status: classified.status,
						message: `[${this.opts.serviceLabel ?? 'API'}] ${classified.message}`,
						service: this.opts.serviceLabel ?? 'API',
						operation: 'request',
						retryable: shouldRetry(error),
						cause: error
					});
				}

				const jitter = Math.floor(Math.random() * API_RESILIENCE.retryJitterMs);
				const waitMs = API_RESILIENCE.baseRetryDelayMs * 2 ** attempt + jitter;
				appState.retryCount = attempt + 1;
				attempt += 1;
				await delay(waitMs);
			}
		}

		throw new Error(
			`[${this.opts.serviceLabel ?? 'API'}] Request failed after ${API_RESILIENCE.maxRetryAttempts} attempts`
		);
	}

	public get<T>(url: string, opts?: { signal?: AbortSignal }): Promise<T> {
		return this.executeWithResilience(() => this.client.get<T>(url, opts));
	}

	public post<T>(url: string, data?: unknown): Promise<T> {
		return this.executeWithResilience(() => this.client.post<T>(url, data));
	}

	public patch<T>(url: string, data?: unknown): Promise<T> {
		return this.executeWithResilience(() => this.client.patch<T>(url, data));
	}

	public delete<T = void>(url: string): Promise<T> {
		return this.executeWithResilience(() => this.client.delete<T>(url));
	}
}

const agentApi = new DashboardApiClient({
	baseUrlGetter: () => appState.agentUrl,
	tokenHeader: 'X-HyperCore-Token',
	tokenGetter: () => appState.agentToken,
	onDisconnect: () => {
		appState.isConnected = false;
	},
	onCircuitChange: (open) => {
		appState.apiCircuitOpen = open;
	},
	serviceLabel: 'AGENT'
});

const launcherApi = new DashboardApiClient({
	baseUrlGetter: () => appState.agentUrl,
	tokenHeader: 'X-HyperCore-Token',
	tokenGetter: () => appState.agentToken,
	onDisconnect: () => {
		appState.launcherConnected = false;
	},
	serviceLabel: 'LAUNCHER'
});

export const AgentRepo = {
	fetchHealth(): Promise<AgentHealth> {
		return agentApi.get<unknown>('/health').then(normalizeAgentHealth);
	},
	fetchMetrics(): Promise<NodeMetrics> {
		return agentApi.get<unknown>('/metrics').then(normalizeMetrics);
	},
	fetchEventsPage(input?: {
		kind?: string;
		action?: string;
		limit?: number;
		cursor?: string;
		fromTs?: string;
	}): Promise<AgentEventsPage> {
		const params = new URLSearchParams();
		if (input?.kind) params.set('kind', input.kind);
		if (input?.action) params.set('action', input.action);
		if (input?.cursor) params.set('cursor', input.cursor);
		if (input?.fromTs) params.set('from_ts', input.fromTs);
		params.set('limit', String(Math.max(1, Math.min(500, Number(input?.limit ?? 80)))));
		const suffix = params.toString();
		return agentApi.get<unknown>(`/events${suffix ? `?${suffix}` : ''}`).then((payload) => {
			const page = normalizeAgentEventsPage(payload);
			if (!page.nextCursor && page.rows.length === page.returned && page.rows.length > 0) {
				return { ...page, nextCursor: page.rows[0]?.id };
			}
			return page;
		});
	},
	async fetchIncidents(): Promise<Incident[]> {
		try {
			const page = await AgentRepo.fetchEventsPage({ limit: 120 });
			const rows = page.rows.map((row, index) => mapAgentEventToIncident(row, index));
			if (rows.length > 0) {
				return rows;
			}
		} catch {
			// Event API may be unavailable on older agents.
		}
		return agentApi.get<unknown>('/state').then(normalizeIncidents);
	},
	executeBlueprint(id: string): Promise<BlueprintRunResult> {
		return agentApi
			.post<unknown>('/blueprints/run', { id })
			.then(normalizeBlueprintRunResult)
			.catch(async () => {
				const response = await AgentRepo.runActionAsync({ action: id, priority: 'normal' });
				return {
					queued: Boolean(response.job?.id),
					jobId: response.job?.id
				};
			});
	},
	fetchPluginHealth(): Promise<PluginHealthMap> {
		return agentApi.get<unknown>('/plugins/health').then(normalizePluginHealthMap);
	},
	fetchBlueprintList(): Promise<Blueprint[]> {
		return agentApi
			.get<unknown>('/blueprints')
			.then(normalizeBlueprintList)
			.catch(async () => {
				const catalog = await AgentRepo.fetchCatalog();
				return catalog.slice(0, 200).map((action, index) => ({
					id: action.id,
					name: action.title,
					category: action.category.toLowerCase().includes('network')
						? 'network'
						: action.category.toLowerCase().includes('security')
							? 'security'
							: 'kernel',
					description: action.desc || `Operation blueprint ${index + 1}`
				}));
			});
	},
	fetchCatalog(): Promise<AgentCatalogAction[]> {
		return agentApi.get<unknown>('/catalog').then(normalizeAgentCatalog);
	},
	requestConfirmation(action: string): Promise<ConfirmationTicket | null> {
		return agentApi
			.post<unknown>('/confirm/request', { action })
			.then(normalizeConfirmationTicket);
	},
	runActionAsync(input: {
		action: string;
		priority?: 'high' | 'normal' | 'low';
		confirmationId?: string;
	}): Promise<AgentRunAsyncResult> {
		const body: Record<string, unknown> = {
			action: input.action,
			priority: input.priority ?? 'normal'
		};
		if (input.confirmationId) {
			body.confirmation_id = input.confirmationId;
		}
		return agentApi.post<unknown>('/run_async', body).then(normalizeRunAsyncResult);
	},
	fetchConfig(): Promise<AgentConfigPayload> {
		return agentApi.get<unknown>('/config').then(normalizeConfigPayload);
	},
	async updateConfig(updates: ConfigUpdateEntry[]): Promise<AgentConfigMutationResult> {
		const applied: ConfigUpdateEntry[] = [];
		for (const entry of updates) {
			const payload = await agentApi.post<unknown>('/config/update', {
				key: entry.path,
				value: entry.value
			});
			const normalized = normalizeConfigMutationResult(payload);
			if (normalized.applied.length > 0) {
				applied.push(...normalized.applied);
			} else {
				applied.push(entry);
			}
		}
		const config = await AgentRepo.fetchConfig().catch(() => undefined);
		return { applied, config };
	},
	applyAutoPreset(mode: 'balanced' | 'fast_dev' | 'reliable_ci'): Promise<AgentConfigMutationResult> {
		return agentApi
			.post<unknown>('/config/auto', { mode })
			.then(normalizeConfigMutationResult);
	},
	applyComposeProfile(input: {
		goal: 'boot_min' | 'linux_full' | 'release_hardening';
		minimal?: boolean;
		noDefaultFeatures?: boolean;
	}): Promise<AgentConfigMutationResult> {
		const body: Record<string, unknown> = {
			goal: input.goal,
			minimal: Boolean(input.minimal ?? false)
		};
		if (typeof input.noDefaultFeatures === 'boolean') {
			body.no_default_features = input.noDefaultFeatures;
		}
		return agentApi
			.post<unknown>('/config/compose/apply', body)
			.then(normalizeConfigMutationResult);
	},
	fetchComposeRecommendation(input: {
		goal: 'boot_min' | 'linux_full' | 'release_hardening';
		minimal?: boolean;
	}): Promise<BuildFeatureRecommendation> {
		const params = new URLSearchParams({ goal: input.goal });
		if (typeof input.minimal === 'boolean') {
			params.set('minimal', String(input.minimal));
		}
		return agentApi
			.get<unknown>(`/config/compose?${params.toString()}`)
			.then((payload) => normalizeBuildFeatureRecommendation((payload as { recommendation?: unknown })?.recommendation));
	},
	fetchConfigDrift(): Promise<ConfigDriftReport> {
		return agentApi
			.get<unknown>('/config/drift')
			.then((payload) => normalizeConfigDriftReport((payload as { drift?: unknown })?.drift));
	},
	applyConfigDrift(input: {
		goal: 'boot_min' | 'linux_full' | 'release_hardening';
		mode: 'full' | 'missing_only';
		minimal?: boolean;
		noDefaultFeatures?: boolean;
	}): Promise<AgentConfigMutationResult> {
		const body: Record<string, unknown> = {
			goal: input.goal,
			mode: input.mode,
			minimal: Boolean(input.minimal ?? false)
		};
		if (typeof input.noDefaultFeatures === 'boolean') {
			body.no_default_features = input.noDefaultFeatures;
		}
		return agentApi
			.post<unknown>('/config/drift/apply', body)
			.then(normalizeConfigMutationResult);
	},
	exportConfigProfile(name: string): Promise<ConfigProfileExport> {
		return agentApi
			.get<unknown>(`/config/export?name=${encodeURIComponent(name)}`)
			.then((payload) => normalizeConfigProfileExport((payload as { profile?: unknown })?.profile));
	},
	fetchConfigOverrideTemplate(mode: 'minimal' | 'full'): Promise<ConfigOverrideTemplate> {
		return agentApi
			.get<unknown>(`/config/overrides/template?mode=${encodeURIComponent(mode)}`)
			.then((payload) => normalizeConfigOverrideTemplate((payload as { template?: unknown })?.template));
	},
	importConfigProfile(profile: unknown): Promise<AgentConfigMutationResult> {
		return agentApi
			.post<unknown>('/config/import', { profile })
			.then(normalizeConfigMutationResult);
	},
	fetchComplianceReport(): Promise<ComplianceReport> {
		return agentApi.get<unknown>('/compliance/report').then(normalizeComplianceReport);
	},
	fetchHosts(): Promise<AgentHost[]> {
		return agentApi.get<unknown>('/hosts').then(normalizeHostList);
	},
	fetchCrashSummary(): Promise<CrashSummary> {
		return agentApi.get<unknown>('/crash/summary').then(normalizeCrashSummary);
	},
	fetchJobs(hostId = 'local'): Promise<AgentJobSummary[]> {
		if (hostId === 'local') {
			return agentApi.get<unknown>('/jobs').then(normalizeJobList);
		}
		const params = new URLSearchParams({ host_id: hostId });
		return agentApi.get<unknown>(`/dispatch/jobs?${params.toString()}`).then(normalizeJobList);
	},
	fetchJob(id: string, tail = 400, hostId = 'local'): Promise<AgentJobDetail> {
		const params = new URLSearchParams({ id, tail: String(tail) });
		if (hostId !== 'local') {
			params.set('host_id', hostId);
			return agentApi.get<unknown>(`/dispatch/job?${params.toString()}`).then(normalizeJobDetail);
		}
		return agentApi.get<unknown>(`/job?${params.toString()}`).then(normalizeJobDetail);
	},
	async streamJob(input: {
		id: string;
		hostId?: string;
		timeoutSec?: number;
		pollMs?: number;
		signal?: AbortSignal;
		onOpen?: () => void;
		onEvent: (event: AgentJobStreamEvent) => void;
		onClose?: () => void;
	}): Promise<void> {
		const pollMs = Math.max(200, Math.min(2000, Number(input.pollMs ?? 600)));
		const hostId = input.hostId && input.hostId.trim() ? input.hostId.trim() : 'local';
		const streamPath = hostId === 'local' ? '/job/events' : '/dispatch/job/events';
		const url = new URL(streamPath, appState.agentUrl);
		url.searchParams.set('id', input.id);
		if (hostId !== 'local') {
			url.searchParams.set('host_id', hostId);
		}
		url.searchParams.set('follow', 'true');
		url.searchParams.set('heartbeat_ms', String(pollMs));

		const headers = new Headers();
		headers.set('Accept', 'text/event-stream');
		if (appState.agentToken) {
			headers.set('X-HyperCore-Token', appState.agentToken);
		}

		let response: Response;
		try {
			response = await fetch(url.toString(), {
				method: 'GET',
				headers,
				signal: input.signal,
				cache: 'no-store'
			});
		} catch (error) {
			if (hostId !== 'local') {
				const snapshot = await AgentRepo.fetchJob(input.id, 400, hostId);
				input.onOpen?.();
				input.onEvent({
					type: 'snapshot',
					detail: snapshot,
					job: snapshot.job,
					lineCount: snapshot.lineCount
				});
				input.onEvent({ type: 'timeout', message: 'Remote stream unavailable, using polling fallback' });
				input.onClose?.();
				return;
			}
			throw error;
		}

		if (!response.ok || !response.body) {
			if (hostId !== 'local') {
				const snapshot = await AgentRepo.fetchJob(input.id, 400, hostId);
				input.onOpen?.();
				input.onEvent({
					type: 'snapshot',
					detail: snapshot,
					job: snapshot.job,
					lineCount: snapshot.lineCount
				});
				input.onEvent({ type: 'timeout', message: `Remote stream failed (${response.status}), using polling fallback` });
				input.onClose?.();
				return;
			}
			throw new Error(`Job stream failed (${response.status})`);
		}

		input.onOpen?.();
		const reader = response.body.getReader();
		const decoder = new TextDecoder();
		let buffer = '';

		while (true) {
			const { done, value } = await reader.read();
			if (done) break;
			buffer += decoder.decode(value, { stream: true });
			while (true) {
				const separatorIndex = buffer.indexOf('\n\n');
				if (separatorIndex < 0) break;
				const rawBlock = buffer.slice(0, separatorIndex);
				buffer = buffer.slice(separatorIndex + 2);
				const parsed = parseSseBlock(rawBlock);
				if (!parsed) continue;
				input.onEvent(toJobStreamEvent(parsed.event, tryParseRecord(parsed.data)));
			}
		}

		const trailing = buffer.trim();
		if (trailing) {
			const parsed = parseSseBlock(trailing);
			if (parsed) {
				input.onEvent(toJobStreamEvent(parsed.event, tryParseRecord(parsed.data)));
			}
		}
		if (!input.signal?.aborted) {
			input.onEvent({ type: 'timeout', message: 'Stream closed by server' });
		}
		input.onClose?.();
	},
	cancelJob(id: string, hostId = 'local'): Promise<AgentJobMutationResult> {
		if (hostId === 'local') {
			return agentApi.post<unknown>('/job/cancel', { id }).then(normalizeJobMutationResult);
		}
		return agentApi
			.post<unknown>('/dispatch/job/cancel', { host_id: hostId, id })
			.then(normalizeJobMutationResult);
	}
};

export const LauncherRepo = {
	async fetchAudit(tail = 40): Promise<AuditLog[]> {
		try {
			return await launcherApi
				.get<unknown>(`/api/launcher/audit?tail=${encodeURIComponent(String(tail))}`)
				.then(normalizeAuditLogs);
		} catch {
			const page = await AgentRepo.fetchEventsPage({ limit: tail });
			return page.rows.map((row, index) => mapAgentEventToAudit(row, index));
		}
	},
	startAgent(): Promise<void> {
		return launcherApi.post('/api/launcher/start-agent');
	},
	stopAgent(): Promise<void> {
		return launcherApi.post('/api/launcher/stop-agent');
	},
	restartAgent(): Promise<void> {
		return launcherApi.post('/api/launcher/restart-agent');
	},
	fetchAgentStatus(): Promise<LauncherStatus> {
		return launcherApi.get<unknown>('/api/launcher/agent-status').then(normalizeLauncherStatus);
	}
};
