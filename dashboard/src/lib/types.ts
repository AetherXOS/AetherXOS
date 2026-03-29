/**
 * @file Core dashboard types
 * Singular source of truth for all domain interfaces.
 */

export type Severity = 'low' | 'medium' | 'high' | 'critical';
export type SyncStatus = 'online' | 'offline' | 'degraded';
export type LocaleCode = 'en' | 'tr';
export type ThemeMode = 'dark' | 'light';
export type StreamEventType = 'audit' | 'incident' | 'status' | 'metrics';

export interface NodeMetrics {
	cpu: number;
	memory: number;
	disk: number;
	uptime: number;
	latency: number;
}

export interface AgentHealth {
	status: string;
	message?: string;
	version?: string;
	connected?: boolean;
}

export interface Incident {
	id: string;
	type: string;
	severity: Severity;
	timestamp: string;
	message: string;
	nodeId: string;
	status: 'open' | 'investigating' | 'resolved';
}

export interface AuditLog {
	id: string;
	timestamp: string;
	action: string;
	operator: string;
	status: 'success' | 'failure';
	details?: string;
}

export interface LauncherStatus {
	status: string;
	state?: string;
	pid?: number | null;
	updatedAt?: string;
}

export interface DashboardSettingsDraft {
	agentUrl: string;
	agentToken: string;
	launcherToken: string;
	syncIntervalMs: number;
	theme: ThemeMode;
	lang: LocaleCode;
}

export interface DashboardSettingsProfileV1 extends DashboardSettingsDraft {
	version?: 1;
}

export interface DashboardSettingsProfileV2 extends DashboardSettingsDraft {
	version: 2;
	exportedAt: string;
}

export type DashboardSettingsProfile = DashboardSettingsProfileV1 | DashboardSettingsProfileV2;

export interface StreamEventEnvelope {
	type: StreamEventType;
	timestamp: string;
	payload: unknown;
}

export interface AuditStreamEvent {
	type: 'audit';
	timestamp: string;
	payload: AuditLog;
}

export interface IncidentStreamEvent {
	type: 'incident';
	timestamp: string;
	payload: Incident;
}

export interface StatusStreamEvent {
	type: 'status';
	timestamp: string;
	payload: LauncherStatus;
}

export interface MetricsStreamEvent {
	type: 'metrics';
	timestamp: string;
	payload: NodeMetrics;
}

export type DashboardStreamEvent =
	| AuditStreamEvent
	| IncidentStreamEvent
	| StatusStreamEvent
	| MetricsStreamEvent;

export interface Blueprint {
	id: string;
	name: string;
	category: 'kernel' | 'network' | 'security';
	description: string;
}

export interface BlueprintRunResult {
	queued: boolean;
	jobId?: string;
}

export type AgentActionRisk = 'INFO' | 'MED' | 'HIGH';

export interface AgentCatalogAction {
	id: string;
	title: string;
	desc: string;
	risk: AgentActionRisk;
	category: string;
	impact?: string;
}

export type ConfigValuePrimitive = string | number | boolean;

export interface ConfigFieldSpec {
	path: string;
	type: 'bool' | 'int' | 'float' | 'string';
	group: string;
	label: string;
	help?: string;
	readonly?: boolean;
	choices?: string[];
	min?: number;
	max?: number;
	meta_source?: string;
}

export interface AgentConfigPayload {
	configPath: string;
	generatedUtc: string;
	values: Record<string, ConfigValuePrimitive>;
	fields: ConfigFieldSpec[];
}

export interface ConfigUpdateEntry {
	path: string;
	value: ConfigValuePrimitive;
}

export interface AgentConfigMutationResult {
	applied: ConfigUpdateEntry[];
	config?: AgentConfigPayload;
	restartHint?: string;
	goal?: string;
	mode?: string;
	minimal?: boolean;
	drift?: ConfigDriftReport;
	recommendation?: BuildFeatureRecommendation;
}

export interface AgentJobSummary {
	id: string;
	action: string;
	title?: string;
	command?: string;
	risk?: AgentActionRisk;
	category?: string;
	status: string;
	priority: 'high' | 'normal' | 'low';
	source?: string;
	ok?: boolean;
	queued_utc?: string;
	started_utc?: string;
	finished_utc?: string;
	exit_code?: number | null;
	queue_wait_ms?: number;
	duration_ms?: number;
	line_count?: number;
	last_poll_utc?: string;
}

export interface AgentHost {
	id: string;
	name: string;
	url: string;
	enabled: boolean;
	roleHint?: string;
	reachable?: boolean;
	busy?: boolean;
	runningCount?: number;
	queueCount?: number;
	role?: string;
	error?: string;
}

export interface AgentRunAsyncResult {
	accepted: boolean;
	job?: AgentJobSummary;
}

export interface AgentJobMutationResult {
	job?: AgentJobSummary;
}

export interface AgentJobDetail {
	hostId?: string;
	job: AgentJobSummary;
	output: string;
	lineCount: number;
}

export interface AgentEventRow {
	id: string;
	kind: string;
	tsUtc: string;
	relatedId?: string;
	action?: string;
	status?: string;
	source?: string;
	detail?: Record<string, unknown>;
}

export interface AgentEventsPage {
	rows: AgentEventRow[];
	nextCursor?: string;
	returned: number;
}

export type AgentJobStreamEventType =
	| 'hello'
	| 'tail'
	| 'heartbeat'
	| 'done'
	| 'timeout'
	| 'error'
	| 'snapshot'
	| 'line'
	| 'complete';

export interface AgentJobStreamEvent {
	type: AgentJobStreamEventType;
	detail?: AgentJobDetail;
	job?: AgentJobSummary;
	lineCount?: number;
	message?: string;
}

export type JobStreamTimelineEventType =
	| AgentJobStreamEventType
	| 'open'
	| 'close'
	| 'reconnect'
	| 'abort';

export interface JobStreamTimelineEvent {
	time: string;
	type: JobStreamTimelineEventType;
	label: string;
}

export interface BuildFeatureRecommendation {
	goal: string;
	minimal: boolean;
	noDefaultFeatures: boolean;
	selectedFeatures: string[];
	selectedCount: number;
	availableCount: number;
	rationale: string[];
}

export interface ConfigDriftGoalRow {
	goal: string;
	recommendedCount: number;
	currentCount: number;
	missingCount: number;
	extraCount: number;
	noDefaultFeaturesRecommended: boolean;
	noDefaultFeaturesCurrent: boolean;
	missing: string[];
	extra: string[];
}

export interface ConfigDriftReport {
	generatedUtc: string;
	current: {
		cargoFeatures: string[];
		cargoNoDefaultFeatures: boolean;
	};
	goals: ConfigDriftGoalRow[];
}

export interface ConfigProfileExport {
	schema: string;
	profileName: string;
	generatedUtc: string;
	sourceConfigPath: string;
	values: Record<string, ConfigValuePrimitive>;
	fieldCount: number;
}

export interface ConfigOverrideTemplate {
	schema: string;
	mode: string;
	generatedUtc: string;
	targetConfigPath: string;
	targetMetaPath: string;
	notes: string[];
	fields: Record<string, Record<string, unknown>>;
	patterns: Record<string, Record<string, unknown>>;
	discoveredFieldCount: number;
}

export interface ComplianceArtifactRow {
	path: string;
	exists: boolean;
	expectedSha256?: string;
	actualSha256?: string;
	checksumMatch?: boolean;
	bytes?: number;
	mtimeUtc?: string;
}

export interface ComplianceCheckRow {
	id: string;
	pass: boolean;
	detail?: string;
}

export interface SecurityRegressionTemplate {
	id: string;
	title: string;
	checks: Array<{
		step: string;
		command: string;
	}>;
}

export interface ComplianceReport {
	generatedUtc: string;
	ok: boolean;
	passCount: number;
	totalChecks: number;
	artifactManifestPath: string;
	artifactVerifyPath: string;
	artifactRows: ComplianceArtifactRow[];
	checks: ComplianceCheckRow[];
	securityRegressionTemplate?: SecurityRegressionTemplate;
}

export interface CrashSummaryEntry {
	id: string;
	path: string;
	exists: boolean;
	ok?: boolean | null;
	modifiedUtc?: string;
}

export interface CrashSummary {
	generatedUtc: string;
	entries: CrashSummaryEntry[];
}

export interface ConfirmationTicket {
	id: string;
	action: string;
	role: string;
	created_utc: string;
	expires_utc: string;
}

export interface PluginHealthEntry {
	status: string;
	version?: string;
	uptime?: number;
	error?: string;
}

export type PluginHealthMap = Record<string, PluginHealthEntry>;

export interface IncidentFilter {
	severity: Severity | 'all';
	status: 'open' | 'investigating' | 'resolved' | 'all';
}
