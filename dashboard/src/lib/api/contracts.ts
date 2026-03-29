import type {
	AgentHealth,
	AgentHost,
	AgentCatalogAction,
	AgentConfigMutationResult,
	AgentConfigPayload,
	AgentEventRow,
	AgentEventsPage,
	AgentJobDetail,
	AgentJobMutationResult,
	AgentJobSummary,
	AgentRunAsyncResult,
	AuditLog,
	Blueprint,
	BuildFeatureRecommendation,
	BlueprintRunResult,
	ComplianceReport,
	ComplianceArtifactRow,
	ConfirmationTicket,
	CrashSummary,
	ConfigDriftGoalRow,
	ConfigDriftReport,
	ConfigFieldSpec,
	ConfigProfileExport,
	ConfigOverrideTemplate,
	ConfigValuePrimitive,
	DashboardStreamEvent,
	Incident,
	LauncherStatus,
	NodeMetrics,
	Severity,
	StreamEventEnvelope,
	StreamEventType
} from '$lib/types';
import type { PluginHealthEntry, PluginHealthMap } from '$lib/types';

const VALID_SEVERITIES: Severity[] = ['low', 'medium', 'high', 'critical'];
const VALID_LAUNCHER_STATUSES = new Set(['online', 'offline', 'starting', 'stopping', 'unknown']);

function asRecord(value: unknown): Record<string, unknown> | null {
	if (value && typeof value === 'object') {
		return value as Record<string, unknown>;
	}
	return null;
}

export function normalizeMetrics(payload: unknown): NodeMetrics {
	const item = asRecord(payload) ?? {};
	const running = Number(item.jobs_running ?? item.running_count ?? 0);
	const queued = Number(item.jobs_queued ?? item.queue_count ?? 0);
	const syntheticCpu = Math.max(0, Math.min(100, running * 25));
	const syntheticMem = Math.max(0, Math.min(100, queued * 10));
	return {
		cpu: Number(item.cpu_load ?? item.cpu ?? syntheticCpu),
		memory: Number(item.mem_pct ?? item.memory ?? syntheticMem),
		disk: Number(item.disk_pct ?? item.disk ?? 0),
		uptime: Number(item.uptime ?? 0),
		latency: Number(item.latency ?? 0)
	};
}

export function normalizeIncident(payload: unknown, index = 0): Incident {
	const item = asRecord(payload) ?? {};
	const severity = VALID_SEVERITIES.includes(item.severity as Severity)
		? (item.severity as Severity)
		: 'medium';
	const status =
		item.status === 'open' || item.status === 'investigating' || item.status === 'resolved'
			? item.status
			: 'open';

	return {
		id: String(item.id ?? `incident-${index}`),
		type: String(item.type ?? 'runtime'),
		severity,
		timestamp: String(item.timestamp ?? new Date().toISOString()),
		message: String(item.message ?? item.summary ?? 'No details available'),
		nodeId: String(item.nodeId ?? item.node ?? 'local'),
		status
	};
}

export function normalizeAuditLog(payload: unknown, index = 0): AuditLog {
	const item = asRecord(payload) ?? {};
	return {
		id: String(item.id ?? `audit-${index}`),
		timestamp: String(item.timestamp ?? item.ts ?? new Date().toISOString()),
		action: String(item.action ?? item.event ?? 'UNKNOWN_ACTION'),
		operator: String(item.operator ?? item.actor ?? 'SYSTEM'),
		status: item.status === 'failure' ? 'failure' : 'success',
		details: item.details ? String(item.details) : undefined
	};
}

export function normalizeIncidents(payload: unknown): Incident[] {
	const item = asRecord(payload);
	const rows = Array.isArray(payload)
		? payload
		: Array.isArray(item?.incidents)
			? item.incidents
			: [];

	return rows.slice(0, 200).map((row, index) => normalizeIncident(row, index));
}

export function normalizeAuditLogs(payload: unknown): AuditLog[] {
	const item = asRecord(payload);
	const rows = Array.isArray(payload) ? payload : Array.isArray(item?.rows) ? item.rows : [];

	return rows.slice(0, 200).map((row, index) => normalizeAuditLog(row, index));
}

export function normalizeLauncherStatus(payload: unknown): LauncherStatus {
	const item = asRecord(payload) ?? {};
	const running = typeof item.running === 'boolean' ? item.running : undefined;
	const rawStatus = String(item.status ?? item.state ?? (running === true ? 'online' : running === false ? 'offline' : 'unknown')).toLowerCase();
	const status = VALID_LAUNCHER_STATUSES.has(rawStatus) ? rawStatus : 'unknown';
	return {
		status,
		state: typeof item.state === 'string' ? item.state : undefined,
		pid: typeof item.pid === 'number' ? item.pid : null,
		updatedAt:
			typeof item.updatedAt === 'string'
				? item.updatedAt
				: typeof item.timestamp === 'string'
					? item.timestamp
					: undefined
	};
}

export function normalizeAgentHealth(payload: unknown): AgentHealth {
	const item = asRecord(payload) ?? {};
	return {
		status: String(item.status ?? item.state ?? 'unknown').toLowerCase(),
		message: typeof item.message === 'string' ? item.message : undefined,
		version: typeof item.version === 'string' ? item.version : undefined,
		connected: typeof item.connected === 'boolean' ? item.connected : undefined
	};
}

export function normalizeHostList(payload: unknown): AgentHost[] {
	const item = asRecord(payload) ?? {};
	const rows = Array.isArray(item.hosts) ? item.hosts : Array.isArray(payload) ? payload : [];
	return rows.slice(0, 100).map((row, index) => {
		const r = asRecord(row) ?? {};
		return {
			id: String(r.id ?? `host-${index}`),
			name: String(r.name ?? r.id ?? `Host ${index + 1}`),
			url: String(r.url ?? ''),
			enabled: r.enabled !== false,
			roleHint: typeof r.role_hint === 'string' ? r.role_hint : undefined,
			reachable: typeof r.reachable === 'boolean' ? r.reachable : undefined,
			busy: typeof r.busy === 'boolean' ? r.busy : undefined,
			runningCount: typeof r.running_count === 'number' ? r.running_count : undefined,
			queueCount: typeof r.queue_count === 'number' ? r.queue_count : undefined,
			role: typeof r.role === 'string' ? r.role : undefined,
			error: typeof r.error === 'string' ? r.error : undefined
		};
	});
}

function getStreamType(item: Record<string, unknown>): StreamEventType {
	const rawType = String(item.type ?? item.event ?? item.kind ?? 'audit').toLowerCase();
	if (rawType.includes('incident') || rawType.includes('alert')) {
		return 'incident';
	}
	if (rawType.includes('status')) {
		return 'status';
	}
	if (rawType.includes('metric')) {
		return 'metrics';
	}
	return 'audit';
}

export function normalizeStreamEvent(payload: unknown): DashboardStreamEvent | null {
	const item = asRecord(payload);
	if (!item) {
		return null;
	}

	const envelope: StreamEventEnvelope = {
		type: getStreamType(item),
		timestamp: typeof item.timestamp === 'string' ? item.timestamp : new Date().toISOString(),
		payload: item.payload ?? item.data ?? item
	};

	if (envelope.type === 'incident') {
		return {
			type: 'incident',
			timestamp: envelope.timestamp,
			payload: normalizeIncident(envelope.payload)
		};
	}

	if (envelope.type === 'status') {
		return {
			type: 'status',
			timestamp: envelope.timestamp,
			payload: normalizeLauncherStatus(envelope.payload)
		};
	}

	if (envelope.type === 'metrics') {
		return {
			type: 'metrics',
			timestamp: envelope.timestamp,
			payload: normalizeMetrics(envelope.payload)
		};
	}

	return {
		type: 'audit',
		timestamp: envelope.timestamp,
		payload: normalizeAuditLog(envelope.payload)
	};
}

export function normalizePluginHealthEntry(payload: unknown): PluginHealthEntry {
	const item = asRecord(payload) ?? {};
	return {
		status: String(item.status ?? 'unknown').toLowerCase(),
		version: typeof item.version === 'string' ? item.version : undefined,
		uptime: typeof item.uptime === 'number' ? item.uptime : undefined,
		error: typeof item.error === 'string' ? item.error : undefined
	};
}

export function normalizePluginHealthMap(payload: unknown): PluginHealthMap {
	const item = asRecord(payload) ?? {};
	const result: PluginHealthMap = {};
	const list = Array.isArray(item.plugins) ? item.plugins : null;
	if (list) {
		for (const row of list) {
			const r = asRecord(row) ?? {};
			const name = String(r.name ?? r.file_name ?? '').trim();
			if (!name) continue;
			result[name] = normalizePluginHealthEntry(r);
		}
		return result;
	}
	for (const [key, val] of Object.entries(item)) {
		if (key === 'ok' || key === 'code' || key === 'message' || key === 'ts_utc') continue;
		result[key] = normalizePluginHealthEntry(val);
	}
	return result;
}

const VALID_BLUEPRINT_CATEGORIES = new Set(['kernel', 'network', 'security']);
const VALID_PRIORITY = new Set(['high', 'normal', 'low']);
const VALID_ACTION_RISK = new Set(['INFO', 'MED', 'HIGH']);

export function normalizeBlueprint(payload: unknown, index = 0): Blueprint {
	const item = asRecord(payload) ?? {};
	const rawCategory = String(item.category ?? item.cat ?? 'kernel').toLowerCase();
	const category = VALID_BLUEPRINT_CATEGORIES.has(rawCategory)
		? (rawCategory as Blueprint['category'])
		: 'kernel';

	return {
		id: String(item.id ?? `blueprint-${index}`),
		name: String(item.name ?? item.label ?? `Blueprint ${index + 1}`),
		category,
		description: String(item.description ?? item.desc ?? 'No description available')
	};
}

export function normalizeBlueprintList(payload: unknown): Blueprint[] {
	const item = asRecord(payload);
	const rows = Array.isArray(payload)
		? payload
		: Array.isArray(item?.blueprints)
			? item.blueprints
			: Array.isArray(item?.rows)
				? item.rows
				: [];

	return rows.slice(0, 200).map((row, index) => normalizeBlueprint(row, index));
}

export function normalizeBlueprintRunResult(payload: unknown): BlueprintRunResult {
	const item = asRecord(payload) ?? {};
	return {
		queued: Boolean(item.queued ?? item.ok ?? true),
		jobId: typeof item.jobId === 'string' ? item.jobId : undefined
	};
}

export function normalizeAgentCatalog(payload: unknown): AgentCatalogAction[] {
	const item = asRecord(payload) ?? {};
	const rows = Array.isArray(item.actions) ? item.actions : [];

	return rows.slice(0, 300).map((row, index) => {
		const r = asRecord(row) ?? {};
		const risk = String(r.risk ?? 'MED').toUpperCase();
		return {
			id: String(r.id ?? `action-${index}`),
			title: String(r.title ?? r.id ?? `Action ${index + 1}`),
			desc: String(r.desc ?? ''),
			risk: VALID_ACTION_RISK.has(risk) ? (risk as AgentCatalogAction['risk']) : 'MED',
			category: String(r.category ?? 'operations'),
			impact: typeof r.impact === 'string' ? r.impact : undefined
		};
	});
}

export function normalizeRunAsyncResult(payload: unknown): AgentRunAsyncResult {
	const item = asRecord(payload) ?? {};
	const jobRaw = asRecord(item.job);
	if (!jobRaw) {
		if (typeof item.id === 'string') {
			const priority = String(item.priority ?? 'normal').toLowerCase();
			const normalizedPriority: AgentJobSummary['priority'] = VALID_PRIORITY.has(priority)
				? (priority as AgentJobSummary['priority'])
				: 'normal';
			return {
				accepted: true,
				job: {
					id: item.id,
					action: String(item.action ?? 'unknown'),
					status: 'queued',
					priority: normalizedPriority
				}
			};
		}
		return {
			accepted: Boolean(item.accepted ?? false),
			job: undefined
		};
	}

	const priority = String(jobRaw.priority ?? 'normal').toLowerCase();
	const normalizedPriority: AgentJobSummary['priority'] = VALID_PRIORITY.has(priority)
		? (priority as AgentJobSummary['priority'])
		: 'normal';

	return {
		accepted: Boolean(item.accepted ?? true),
		job: {
			id: String(jobRaw.id ?? 'job-unknown'),
			action: String(jobRaw.action ?? 'unknown'),
			status: String(jobRaw.status ?? 'queued'),
			priority: normalizedPriority,
			queued_utc: typeof jobRaw.queued_utc === 'string' ? jobRaw.queued_utc : undefined,
			started_utc: typeof jobRaw.started_utc === 'string' ? jobRaw.started_utc : undefined,
			finished_utc: typeof jobRaw.finished_utc === 'string' ? jobRaw.finished_utc : undefined,
			exit_code: typeof jobRaw.exit_code === 'number' ? jobRaw.exit_code : undefined
		}
	};
}

export function normalizeJobSummary(payload: unknown): AgentJobSummary {
	const item = asRecord(payload) ?? {};
	const priority = String(item.priority ?? 'normal').toLowerCase();
	const normalizedPriority: AgentJobSummary['priority'] = VALID_PRIORITY.has(priority)
		? (priority as AgentJobSummary['priority'])
		: 'normal';
	const risk = String(item.risk ?? '').toUpperCase();

	return {
		id: String(item.id ?? 'job-unknown'),
		action: String(item.action ?? 'unknown'),
		title: typeof item.title === 'string' ? item.title : undefined,
		command: typeof item.command === 'string' ? item.command : undefined,
		risk: VALID_ACTION_RISK.has(risk) ? (risk as AgentJobSummary['risk']) : undefined,
		category: typeof item.category === 'string' ? item.category : undefined,
		status: String(item.status ?? 'queued'),
		priority: normalizedPriority,
		source: typeof item.source === 'string' ? item.source : undefined,
		ok: typeof item.ok === 'boolean' ? item.ok : undefined,
		queued_utc: typeof item.queued_utc === 'string' ? item.queued_utc : undefined,
		started_utc: typeof item.started_utc === 'string' ? item.started_utc : undefined,
		finished_utc: typeof item.finished_utc === 'string' ? item.finished_utc : undefined,
		exit_code: typeof item.exit_code === 'number' ? item.exit_code : undefined,
		queue_wait_ms: typeof item.queue_wait_ms === 'number' ? item.queue_wait_ms : undefined,
		duration_ms: typeof item.duration_ms === 'number' ? item.duration_ms : undefined,
		line_count: typeof item.line_count === 'number' ? item.line_count : undefined,
		last_poll_utc: typeof item.last_poll_utc === 'string' ? item.last_poll_utc : undefined
	};
}

export function normalizeJobList(payload: unknown): AgentJobSummary[] {
	const item = asRecord(payload) ?? {};
	const rows = Array.isArray(item.jobs) ? item.jobs : Array.isArray(payload) ? payload : [];
	return rows.map((row) => normalizeJobSummary(row));
}

export function normalizeJobMutationResult(payload: unknown): AgentJobMutationResult {
	const item = asRecord(payload) ?? {};
	const jobRaw = asRecord(item.job);
	return {
		job: jobRaw ? normalizeJobSummary(jobRaw) : undefined
	};
}

export function normalizeJobDetail(payload: unknown): AgentJobDetail {
	const item = asRecord(payload) ?? {};
	const jobRaw = asRecord(item.job) ?? {};
	const output = typeof item.output === 'string'
		? item.output
		: Array.isArray(jobRaw.output)
			? jobRaw.output.map((line) => String(line)).join('\n')
			: '';
	return {
		hostId: typeof item.host_id === 'string' ? item.host_id : undefined,
		job: normalizeJobSummary(jobRaw),
		output,
		lineCount: Number(item.line_count ?? jobRaw.line_count ?? (Array.isArray(jobRaw.output) ? jobRaw.output.length : 0))
	};
}

function normalizeEventRow(payload: unknown, index = 0): AgentEventRow {
	const item = asRecord(payload) ?? {};
	return {
		id: String(item.id ?? `event-${index}`),
		kind: String(item.kind ?? item.type ?? 'unknown').toLowerCase(),
		tsUtc: String(item.ts_utc ?? item.timestamp ?? new Date().toISOString()),
		relatedId: typeof item.related_id === 'string' ? item.related_id : undefined,
		action: typeof item.action === 'string' ? item.action : undefined,
		status: typeof item.status === 'string' ? item.status : undefined,
		source: typeof item.source === 'string' ? item.source : undefined,
		detail: asRecord(item.detail) ?? undefined
	};
}

export function normalizeAgentEventsPage(payload: unknown): AgentEventsPage {
	const item = asRecord(payload) ?? {};
	const rowsRaw = Array.isArray(item.events)
		? item.events
		: Array.isArray(item.rows)
			? item.rows
			: Array.isArray(payload)
				? payload
				: [];
	const rows = rowsRaw.map((row, index) => normalizeEventRow(row, index));
	const nextCursor = typeof item.next_cursor === 'string' ? item.next_cursor : undefined;
	const returned = typeof item.returned === 'number' ? item.returned : rows.length;
	return { rows, nextCursor, returned };
}

function inferIncidentSeverity(event: AgentEventRow): Incident['severity'] {
	const probe = `${event.kind} ${event.status ?? ''} ${event.action ?? ''}`.toLowerCase();
	if (probe.includes('panic') || probe.includes('failed') || probe.includes('error')) {
		return 'high';
	}
	if (probe.includes('cancel') || probe.includes('degraded')) {
		return 'medium';
	}
	return 'low';
}

export function mapAgentEventToIncident(event: AgentEventRow, index = 0): Incident {
	const status: Incident['status'] =
		event.status === 'resolved' ? 'resolved' : event.status === 'investigating' ? 'investigating' : 'open';
	return {
		id: event.id || `incident-${index}`,
		type: event.kind || 'runtime',
		severity: inferIncidentSeverity(event),
		timestamp: event.tsUtc,
		message: event.action ?? event.kind ?? 'Event',
		nodeId: event.source ?? 'local',
		status
	};
}

export function mapAgentEventToAudit(event: AgentEventRow, index = 0): AuditLog {
	const failed = (event.status ?? '').toLowerCase();
	return {
		id: event.id || `audit-${index}`,
		timestamp: event.tsUtc,
		action: event.action ?? event.kind ?? 'EVENT',
		operator: event.source ?? 'AGENT',
		status: failed.includes('fail') || failed.includes('error') ? 'failure' : 'success',
		details: event.relatedId
	};
}

export function normalizeConfirmationTicket(payload: unknown): ConfirmationTicket | null {
	const item = asRecord(payload) ?? {};
	const raw = asRecord(item.confirmation);
	if (!raw) {
		return null;
	}

	return {
		id: String(raw.id ?? ''),
		action: String(raw.action ?? ''),
		role: String(raw.role ?? ''),
		created_utc: String(raw.created_utc ?? ''),
		expires_utc: String(raw.expires_utc ?? '')
	};
}

function normalizeConfigValuePrimitive(value: unknown): ConfigValuePrimitive {
	if (typeof value === 'boolean' || typeof value === 'number' || typeof value === 'string') {
		return value;
	}
	return String(value ?? '');
}

function normalizeConfigFieldSpec(payload: unknown): ConfigFieldSpec {
	const item = asRecord(payload) ?? {};
	const rawType = String(item.type ?? 'string').toLowerCase();
	const type: ConfigFieldSpec['type'] =
		rawType === 'bool' || rawType === 'int' || rawType === 'float' ? rawType : 'string';

	return {
		path: String(item.path ?? ''),
		type,
		group: String(item.group ?? 'General'),
		label: String(item.label ?? item.path ?? 'Config Field'),
		help: typeof item.help === 'string' ? item.help : undefined,
		readonly: Boolean(item.readonly ?? false),
		choices: Array.isArray(item.choices)
			? item.choices.map((choice) => String(choice))
			: undefined,
		min: typeof item.min === 'number' ? item.min : undefined,
		max: typeof item.max === 'number' ? item.max : undefined,
		meta_source: typeof item.meta_source === 'string' ? item.meta_source : undefined
	};
}

export function normalizeConfigPayload(payload: unknown): AgentConfigPayload {
	const item = asRecord(payload) ?? {};
	const wrappedConfig = asRecord(item.config);
	const agentScoped = asRecord(wrappedConfig?.agent ?? item.agent);
	const valuesRaw = asRecord(item.values) ?? agentScoped ?? wrappedConfig ?? {};
	const values: Record<string, ConfigValuePrimitive> = {};
	for (const [key, value] of Object.entries(valuesRaw)) {
		values[key] = normalizeConfigValuePrimitive(value);
	}

	const fieldsRaw = Array.isArray(item.fields) ? item.fields : [];
	const fields = fieldsRaw
		.map((field) => normalizeConfigFieldSpec(field))
		.filter((field) => field.path.length > 0);

	return {
		configPath: String(item.config_path ?? item.source_config_path ?? ''),
		generatedUtc: String(item.generated_utc ?? new Date().toISOString()),
		values,
		fields
	};
}

export function normalizeConfigMutationResult(payload: unknown): AgentConfigMutationResult {
	const item = asRecord(payload) ?? {};
	const appliedRows = Array.isArray(item.applied) ? item.applied : [];
	const applied = appliedRows.map((row) => {
		const r = asRecord(row) ?? {};
		return {
			path: String(r.path ?? ''),
			value: normalizeConfigValuePrimitive(r.value)
		};
	});
	if (applied.length === 0 && typeof item.key === 'string') {
		applied.push({
			path: item.key,
			value: normalizeConfigValuePrimitive(item.value)
		});
	}

	const configRaw = asRecord(item.config);
	return {
		applied,
		config: configRaw ? normalizeConfigPayload(configRaw) : undefined,
		restartHint: typeof item.restart_hint === 'string' ? item.restart_hint : undefined,
		goal: typeof item.goal === 'string' ? item.goal : undefined,
		mode: typeof item.mode === 'string' ? item.mode : undefined,
		minimal: typeof item.minimal === 'boolean' ? item.minimal : undefined,
		drift: asRecord(item.drift) ? normalizeConfigDriftReport(item.drift) : undefined,
		recommendation: asRecord(item.recommendation)
			? normalizeBuildFeatureRecommendation(item.recommendation)
			: undefined
	};
}

export function normalizeBuildFeatureRecommendation(payload: unknown): BuildFeatureRecommendation {
	const item = asRecord(payload) ?? {};
	return {
		goal: String(item.goal ?? 'linux_full'),
		minimal: Boolean(item.minimal ?? false),
		noDefaultFeatures: Boolean(item.no_default_features ?? false),
		selectedFeatures: Array.isArray(item.selected_features)
			? item.selected_features.map((value) => String(value))
			: [],
		selectedCount: Number(item.selected_count ?? 0),
		availableCount: Number(item.available_count ?? 0),
		rationale: Array.isArray(item.rationale)
			? item.rationale.map((value) => String(value))
			: []
	};
}

function normalizeConfigDriftGoalRow(payload: unknown): ConfigDriftGoalRow {
	const item = asRecord(payload) ?? {};
	return {
		goal: String(item.goal ?? 'linux_full'),
		recommendedCount: Number(item.recommended_count ?? 0),
		currentCount: Number(item.current_count ?? 0),
		missingCount: Number(item.missing_count ?? 0),
		extraCount: Number(item.extra_count ?? 0),
		noDefaultFeaturesRecommended: Boolean(item.no_default_features_recommended ?? false),
		noDefaultFeaturesCurrent: Boolean(item.no_default_features_current ?? false),
		missing: Array.isArray(item.missing) ? item.missing.map((value) => String(value)) : [],
		extra: Array.isArray(item.extra) ? item.extra.map((value) => String(value)) : []
	};
}

export function normalizeConfigDriftReport(payload: unknown): ConfigDriftReport {
	const item = asRecord(payload) ?? {};
	const current = asRecord(item.current) ?? {};
	return {
		generatedUtc: String(item.generated_utc ?? new Date().toISOString()),
		current: {
			cargoFeatures: Array.isArray(current.cargo_features)
				? current.cargo_features.map((value) => String(value))
				: [],
			cargoNoDefaultFeatures: Boolean(current.cargo_no_default_features ?? false)
		},
		goals: Array.isArray(item.goals) ? item.goals.map((goal) => normalizeConfigDriftGoalRow(goal)) : []
	};
}

export function normalizeConfigProfileExport(payload: unknown): ConfigProfileExport {
	const item = asRecord(payload) ?? {};
	const valuesRaw = asRecord(item.values) ?? {};
	const values: Record<string, ConfigValuePrimitive> = {};
	for (const [key, value] of Object.entries(valuesRaw)) {
		values[key] = normalizeConfigValuePrimitive(value);
	}
	return {
		schema: String(item.schema ?? ''),
		profileName: String(item.profile_name ?? 'default'),
		generatedUtc: String(item.generated_utc ?? new Date().toISOString()),
		sourceConfigPath: String(item.source_config_path ?? ''),
		values,
		fieldCount: Number(item.field_count ?? Object.keys(values).length)
	};
}

export function normalizeConfigOverrideTemplate(payload: unknown): ConfigOverrideTemplate {
	const item = asRecord(payload) ?? {};
	const fieldsRaw = asRecord(item.fields) ?? {};
	const patternsRaw = asRecord(item.patterns) ?? {};
	const fields: Record<string, Record<string, unknown>> = {};
	const patterns: Record<string, Record<string, unknown>> = {};
	for (const [key, value] of Object.entries(fieldsRaw)) {
		fields[key] = asRecord(value) ?? {};
	}
	for (const [key, value] of Object.entries(patternsRaw)) {
		patterns[key] = asRecord(value) ?? {};
	}
	return {
		schema: String(item.schema ?? ''),
		mode: String(item.mode ?? 'minimal'),
		generatedUtc: String(item.generated_utc ?? new Date().toISOString()),
		targetConfigPath: String(item.target_config_path ?? ''),
		targetMetaPath: String(item.target_meta_path ?? ''),
		notes: Array.isArray(item.notes) ? item.notes.map((value) => String(value)) : [],
		fields,
		patterns,
		discoveredFieldCount: Number(item.discovered_field_count ?? 0)
	};
}

function normalizeComplianceArtifactRow(payload: unknown): ComplianceArtifactRow {
	const item = asRecord(payload) ?? {};
	return {
		path: String(item.path ?? ''),
		exists: Boolean(item.exists ?? false),
		expectedSha256: typeof item.expected_sha256 === 'string' ? item.expected_sha256 : undefined,
		actualSha256: typeof item.actual_sha256 === 'string' ? item.actual_sha256 : undefined,
		checksumMatch: typeof item.checksum_match === 'boolean' ? item.checksum_match : undefined,
		bytes: typeof item.bytes === 'number' ? item.bytes : undefined,
		mtimeUtc: typeof item.mtime_utc === 'string' ? item.mtime_utc : undefined
	};
}

export function normalizeComplianceReport(payload: unknown): ComplianceReport {
	const item = asRecord(payload) ?? {};
	const checksRaw = Array.isArray(item.checks) ? item.checks : [];
	const artifactsRaw = Array.isArray(item.artifact_rows) ? item.artifact_rows : [];
	const templateRaw = asRecord(item.security_regression_template);
	return {
		generatedUtc: String(item.generated_utc ?? new Date().toISOString()),
		ok: Boolean(item.ok ?? false),
		passCount: Number(item.pass_count ?? 0),
		totalChecks: Number(item.total_checks ?? 0),
		artifactManifestPath: String(item.artifact_manifest_path ?? ''),
		artifactVerifyPath: String(item.artifact_verify_path ?? ''),
		artifactRows: artifactsRaw.map((row) => normalizeComplianceArtifactRow(row)),
		checks: checksRaw.map((check) => {
			const c = asRecord(check) ?? {};
			return {
				id: String(c.id ?? ''),
				pass: Boolean(c.pass ?? false),
				detail: typeof c.detail === 'string' ? c.detail : undefined
			};
		}),
		securityRegressionTemplate: templateRaw
			? {
				id: String(templateRaw.id ?? ''),
				title: String(templateRaw.title ?? ''),
				checks: Array.isArray(templateRaw.checks)
					? templateRaw.checks.map((row) => {
						const r = asRecord(row) ?? {};
						return {
							step: String(r.step ?? ''),
							command: String(r.command ?? '')
						};
					})
					: []
			}
			: undefined
	};
}

export function normalizeCrashSummary(payload: unknown): CrashSummary {
	const item = asRecord(payload) ?? {};
	const entriesRaw = Array.isArray(item.entries)
		? item.entries
		: typeof item.crash_count === 'number'
			? [
				{
					id: 'crash-artifacts',
					path: String(item.artifacts_dir ?? 'artifacts/crash'),
					exists: item.crash_count > 0,
					ok: item.crash_count === 0
				}
			]
			: [];
	return {
		generatedUtc: String(item.generated_utc ?? new Date().toISOString()),
		entries: entriesRaw.map((row) => {
			const e = asRecord(row) ?? {};
			return {
				id: String(e.id ?? ''),
				path: String(e.path ?? ''),
				exists: Boolean(e.exists ?? false),
				ok: typeof e.ok === 'boolean' ? e.ok : null,
				modifiedUtc: typeof e.modified_utc === 'string' ? e.modified_utc : undefined
			};
		})
	};
}
