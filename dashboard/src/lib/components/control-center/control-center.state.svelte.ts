import { AgentRepo, LauncherRepo } from '$lib/api';
import { CONTROL_CENTER } from '$lib/config/runtime-client';
import { m } from '$lib/paraglide/messages';
import { appState } from '$lib/state.svelte';
import type {
	AgentCatalogAction,
	AgentConfigPayload,
	AgentHost,
	AgentJobDetail,
	AgentJobStreamEvent,
	AgentJobSummary,
	Blueprint,
	JobStreamTimelineEvent,
	BuildFeatureRecommendation,
	ComplianceReport,
	ConfigDriftReport,
	ConfigFieldSpec,
	ConfigProfileExport,
	ConfigOverrideTemplate,
	ConfigUpdateEntry,
	ConfigValuePrimitive,
	CrashSummary
} from '$lib/types';
import type {
	AutoPresetMode,
	ComposeGoal,
	ControlPriority,
	ControlTab,
	DriftApplyMode,
	LauncherAction,
	OverrideTemplateMode
} from './types';
import {
	cancelJobDomain,
	cancelSelectedJobDomain,
	fetchJobDetailDomain,
	refreshJobsDomain,
	selectJobDomain,
	selectJobHostDomain,
	setJobAutoRefreshEnabledDomain,
	setJobAutoRefreshMsDomain,
	setJobStreamEnabledDomain,
	setSelectedJobTailDomain
} from './state/jobs-domain';
import {
	applyAutoPresetDomain,
	applyComposeProfileDomain,
	applyConfigUpdatesDomain,
	applyMutationSnapshotDomain,
	getPendingConfigUpdatesDomain,
	isFieldChangedDomain,
	parseDraftValueDomain,
	refreshConfigDomain,
	toDraftTextDomain
} from './state/config-domain';
import {
	applyDriftFixDomain,
	exportProfileDomain,
	importProfileDomain,
	refreshBuildInsightsDomain,
	refreshComposeRecommendationDomain,
	refreshOverrideTemplateDomain
} from './state/build-domain';

export class ControlCenterState {
	private jobAutoRefreshTimer: ReturnType<typeof setInterval> | null = null;
	private jobStreamAbortController: AbortController | null = null;
	private jobStreamReconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private jobStreamTerminal = false;

	public activeTab = $state<ControlTab>('operations');
	public isExecuting = $state(false);
	public launcherBusy = $state(false);
	public operationsBusy = $state(false);
	public configBusy = $state(false);
	public buildBusy = $state(false);
	public jobsBusy = $state(false);
	public jobDetailBusy = $state(false);
	public operationPriority = $state<ControlPriority>('normal');
	public operationMessage = $state('');
	public operationSearch = $state('');
	public operationCategory = $state('all');
	public configMessage = $state('');
	public buildMessage = $state('');
	public configSearch = $state('');
	public showChangedOnly = $state(false);
	public autoPresetMode = $state<AutoPresetMode>('balanced');
	public composeGoal = $state<ComposeGoal>('linux_full');
	public composeMinimal = $state(false);
	public driftApplyMode = $state<DriftApplyMode>('missing_only');
	public overrideTemplateMode = $state<OverrideTemplateMode>('minimal');
	public exportProfileName = $state('default');
	public importProfileText = $state('');
	public selectedJobId = $state('');
	public selectedJobHostId = $state('local');
	public selectedJobTail = $state(400);
	public jobAutoRefreshEnabled = $state(true);
	public jobAutoRefreshMs = $state(5000);
	public jobStreamEnabled = $state(true);
	public jobStreamConnected = $state(false);
	public jobStreamStatus = $state<'idle' | 'connecting' | 'streaming' | 'fallback' | 'error'>('idle');
	public streamEvents = $state<JobStreamTimelineEvent[]>([]);
	public streamReconnectCount = $state(0);
	public blueprints = $state<Blueprint[]>([]);
	public operationCatalog = $state<AgentCatalogAction[]>([]);
	public hostRows = $state<AgentHost[]>([]);
	public jobRows = $state<AgentJobSummary[]>([]);
	public pendingAction = $state<AgentCatalogAction | null>(null);
	public configSnapshot = $state<AgentConfigPayload | null>(null);
	public configDraft = $state<Record<string, string>>({});
	public composeRecommendation = $state<BuildFeatureRecommendation | null>(null);
	public driftReport = $state<ConfigDriftReport | null>(null);
	public exportedProfile = $state<ConfigProfileExport | null>(null);
	public overrideTemplate = $state<ConfigOverrideTemplate | null>(null);
	public selectedJobDetail = $state<AgentJobDetail | null>(null);
	public complianceReport = $state<ComplianceReport | null>(null);
	public crashSummary = $state<CrashSummary | null>(null);

	public tabs = $derived.by(() => [
		{ id: 'operations' as const, label: m.control_tab_operations() },
		{ id: 'build' as const, label: 'Build' },
		{ id: 'config' as const, label: 'Config' },
		{ id: 'blueprints' as const, label: m.control_tab_blueprints() },
		{ id: 'plugins' as const, label: m.control_tab_plugins() },
		{ id: 'launcher' as const, label: m.control_tab_launcher() }
	]);

	public fallbackBlueprints = $derived<Blueprint[]>([
		{
			id: 'BL_KERNEL_GUARD',
			name: m.blueprint_kernel_guard_name(),
			category: 'kernel',
			description: m.blueprint_kernel_guard_desc()
		},
		{
			id: 'BL_NETWORK_MESH',
			name: m.blueprint_network_mesh_name(),
			category: 'network',
			description: m.blueprint_network_mesh_desc()
		},
		{
			id: 'BL_SECURITY_ATTEST',
			name: m.blueprint_security_attestation_name(),
			category: 'security',
			description: m.blueprint_security_attestation_desc()
		}
	]);

	public fallbackOps = $derived<AgentCatalogAction[]>([
		{
			id: 'install_deno',
			title: m.control_op_install_deno_title(),
			desc: m.control_op_install_deno_desc(),
			risk: 'MED',
			category: 'install',
			impact: m.control_op_install_deno_impact()
		},
		{
			id: 'doctor_fix',
			title: m.control_op_doctor_fix_title(),
			desc: m.control_op_doctor_fix_desc(),
			risk: 'HIGH',
			category: 'install'
		},
		{
			id: 'build_iso',
			title: m.control_op_build_iso_title(),
			desc: m.control_op_build_iso_desc(),
			risk: 'HIGH',
			category: 'build'
		},
		{
			id: 'qemu_smoke',
			title: m.control_op_qemu_smoke_title(),
			desc: m.control_op_qemu_smoke_desc(),
			risk: 'HIGH',
			category: 'test'
		},
		{
			id: 'qemu_live',
			title: m.control_op_qemu_live_title(),
			desc: m.control_op_qemu_live_desc(),
			risk: 'MED',
			category: 'test'
		},
		{
			id: 'dashboard_build',
			title: m.control_op_dashboard_build_title(),
			desc: m.control_op_dashboard_build_desc(),
			risk: 'MED',
			category: 'dashboard'
		},
		{
			id: 'dashboard_tests',
			title: m.control_op_dashboard_tests_title(),
			desc: m.control_op_dashboard_tests_desc(),
			risk: 'MED',
			category: 'test'
		},
		{
			id: 'quality_gate',
			title: m.control_op_quality_gate_title(),
			desc: m.control_op_quality_gate_desc(),
			risk: 'HIGH',
			category: 'gate'
		},
		{
			id: 'open_report',
			title: m.control_op_open_report_title(),
			desc: m.control_op_open_report_desc(),
			risk: 'INFO',
			category: 'dashboard'
		}
	]);

	public operationRows = $derived.by(() => {
		const rows = this.operationCatalog.length > 0 ? this.operationCatalog : this.fallbackOps;
		const search = this.operationSearch.trim().toLowerCase();
		return rows.filter((row) => {
			if (this.operationCategory !== 'all' && row.category !== this.operationCategory) return false;
			if (!search) return true;
			const probe = `${row.title} ${row.desc} ${row.category} ${row.id}`.toLowerCase();
			return probe.includes(search);
		});
	});

	public operationCategories = $derived.by(() => {
		const rows = this.operationCatalog.length > 0 ? this.operationCatalog : this.fallbackOps;
		const seen: Record<string, true> = {};
		for (const row of rows) seen[row.category] = true;
		return Object.keys(seen).sort();
	});

	public filteredConfigFields = $derived.by(() => {
		const fields = this.configSnapshot?.fields ?? [];
		const query = this.configSearch.trim().toLowerCase();
		return fields.filter((field) => {
			if (this.showChangedOnly && !this.isFieldChanged(field)) return false;
			if (!query) return true;
			const probe = `${field.path} ${field.label} ${field.group} ${field.help ?? ''}`.toLowerCase();
			return probe.includes(query);
		});
	});

	public configGroups = $derived.by(() => {
		const grouped: Record<string, ConfigFieldSpec[]> = {};
		for (const field of this.filteredConfigFields) {
			const key = field.group || 'General';
			const bucket = grouped[key] ?? [];
			bucket.push(field);
			grouped[key] = bucket;
		}
		return Object.entries(grouped).sort(([a], [b]) => a.localeCompare(b));
	});

	public categoryLabel = $derived.by<Record<Blueprint['category'], string>>(() => ({
		kernel: m.blueprint_cat_kernel(),
		network: m.blueprint_cat_network(),
		security: m.blueprint_cat_security()
	}));

	public pluginRows = $derived(
		Object.entries(appState.pluginHealth).sort((a, b) => a[0].localeCompare(b[0]))
	);
	public pendingConfigCount = $derived.by(() => this.getPendingConfigUpdates().length);
	public isoArtifactRows = $derived.by(() =>
		(this.complianceReport?.artifactRows ?? []).filter((row) => row.path.toLowerCase().includes('.iso'))
	);

	private loadBlueprintCache(): Blueprint[] | null {
		if (typeof window === 'undefined') return null;
		try {
			const raw = window.localStorage.getItem(CONTROL_CENTER.blueprintCacheKey);
			if (!raw) return null;
			const parsed = JSON.parse(raw) as { expiresAt?: number; rows?: Blueprint[] };
			if (typeof parsed.expiresAt !== 'number' || Date.now() > parsed.expiresAt) {
				window.localStorage.removeItem(CONTROL_CENTER.blueprintCacheKey);
				return null;
			}
			return Array.isArray(parsed.rows) ? parsed.rows : null;
		} catch {
			return null;
		}
	}

	private persistBlueprintCache(rows: Blueprint[]): void {
		if (typeof window === 'undefined') return;
		window.localStorage.setItem(
			CONTROL_CENTER.blueprintCacheKey,
			JSON.stringify({ expiresAt: Date.now() + CONTROL_CENTER.blueprintCacheTtlMs, rows })
		);
	}

	private pushStreamEvent(type: JobStreamTimelineEvent['type'], label: string): void {
		const time = new Intl.DateTimeFormat('en', {
			hour12: false,
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		}).format(Date.now());
		this.streamEvents = [...this.streamEvents.slice(-49), { time, type, label }];
	}

	private stopJobAutoRefresh(): void {
		if (this.jobAutoRefreshTimer) {
			clearInterval(this.jobAutoRefreshTimer);
			this.jobAutoRefreshTimer = null;
		}
	}

	private clearJobStreamReconnectTimer(): void {
		if (this.jobStreamReconnectTimer) {
			clearTimeout(this.jobStreamReconnectTimer);
			this.jobStreamReconnectTimer = null;
		}
	}

	private stopJobStream(): void {
		this.clearJobStreamReconnectTimer();
		if (this.jobStreamAbortController) {
			this.jobStreamAbortController.abort();
			this.jobStreamAbortController = null;
		}
		this.jobStreamConnected = false;
		if (this.jobStreamStatus === 'connecting' || this.jobStreamStatus === 'streaming') {
			this.jobStreamStatus = 'idle';
		}
	}

	private startJobAutoRefresh(): void {
		this.stopJobAutoRefresh();
		if (!this.jobAutoRefreshEnabled) return;
		this.jobAutoRefreshTimer = setInterval(() => {
			if (!this.jobAutoRefreshEnabled || !this.selectedJobId) return;
			void this.fetchJobDetail(this.selectedJobId, this.selectedJobTail, true);
		}, Math.max(1500, this.jobAutoRefreshMs));
	}

	private upsertJobSummary(summary: AgentJobSummary): void {
		this.jobRows = this.jobRows.map((row) => (row.id === summary.id ? { ...row, ...summary } : row));
		if (!this.selectedJobDetail || this.selectedJobDetail.job.id !== summary.id) {
			return;
		}
		this.selectedJobDetail = {
			...this.selectedJobDetail,
			job: { ...this.selectedJobDetail.job, ...summary },
			lineCount: summary.line_count ?? this.selectedJobDetail.lineCount
		};
	}

	private applyStreamEvent(event: AgentJobStreamEvent): void {
		if (event.detail && event.detail.job.id === this.selectedJobId) {
			this.selectedJobDetail = event.detail;
		}
		if (event.type === 'snapshot' && event.detail) {
			this.selectedJobDetail = event.detail;
			this.jobStreamConnected = true;
			this.jobStreamStatus = 'streaming';
			this.stopJobAutoRefresh();
			this.pushStreamEvent('tail', `snapshot (${event.lineCount ?? event.detail.lineCount ?? 0} lines)`);
			return;
		}
		if (event.type === 'line' && this.selectedJobDetail && this.selectedJobDetail.job.id === this.selectedJobId) {
			const current = this.selectedJobDetail.output;
			const appended = event.message ?? '';
			this.selectedJobDetail = {
				...this.selectedJobDetail,
				output: current ? `${current}\n${appended}` : appended,
				lineCount: event.lineCount ?? this.selectedJobDetail.lineCount + 1
			};
			this.jobStreamConnected = true;
			this.jobStreamStatus = 'streaming';
			this.stopJobAutoRefresh();
			this.pushStreamEvent('tail', `line (${this.selectedJobDetail.lineCount} lines)`);
			return;
		}
		if (event.job) {
			this.upsertJobSummary(event.job);
		}
		if (event.type === 'tail' || event.type === 'heartbeat') {
			this.jobStreamConnected = true;
			this.jobStreamStatus = 'streaming';
			this.stopJobAutoRefresh();
			if (event.type === 'tail') this.pushStreamEvent('tail', `tail (${event.lineCount ?? 0} lines)`);
			else this.pushStreamEvent('heartbeat', 'heartbeat');
			return;
		}
		if (event.type === 'done') {
			this.jobStreamConnected = false;
			this.jobStreamStatus = 'idle';
			this.jobStreamTerminal = true;
			this.stopJobAutoRefresh();
			this.pushStreamEvent('done', 'Job done');
			return;
		}
		if (event.type === 'complete') {
			this.jobStreamConnected = false;
			this.jobStreamStatus = 'idle';
			this.jobStreamTerminal = true;
			this.stopJobAutoRefresh();
			this.pushStreamEvent('done', 'Job complete');
			return;
		}
		if (event.type === 'timeout') {
			this.jobStreamConnected = false;
			this.jobStreamStatus = 'fallback';
			if (this.jobAutoRefreshEnabled) this.startJobAutoRefresh();
			this.pushStreamEvent('timeout', 'Stream timeout → fallback');
			return;
		}
		if (event.type === 'error') {
			this.jobStreamConnected = false;
			this.jobStreamStatus = 'error';
			if (this.jobAutoRefreshEnabled) this.startJobAutoRefresh();
			this.pushStreamEvent('error', event.message ?? 'Stream error');
		}
	}

	private scheduleJobStreamReconnect(): void {
		this.clearJobStreamReconnectTimer();
		this.streamReconnectCount += 1;
		this.pushStreamEvent('reconnect', `Reconnecting (#${this.streamReconnectCount})…`);
		this.jobStreamReconnectTimer = setTimeout(() => {
			if (!this.jobStreamEnabled || !this.selectedJobId || this.jobStreamTerminal) return;
			this.startJobStream();
		}, 1500);
	}

	private startJobStream(): void {
		this.stopJobStream();
		if (!this.jobStreamEnabled || !this.selectedJobId) return;

		const controller = new AbortController();
		const targetJobId = this.selectedJobId;
		this.jobStreamTerminal = false;
		this.jobStreamAbortController = controller;
		this.jobStreamConnected = false;
		this.jobStreamStatus = 'connecting';

		this.pushStreamEvent('open', `Connecting to ${this.selectedJobHostId !== 'local' ? 'dispatch/' + this.selectedJobHostId : 'local'}`);
		void AgentRepo.streamJob({
			id: targetJobId,
			hostId: this.selectedJobHostId,
			timeoutSec: 180,
			pollMs: 600,
			signal: controller.signal,
			onOpen: () => {
				if (this.jobStreamAbortController !== controller) return;
				this.jobStreamConnected = true;
				this.jobStreamStatus = 'streaming';
				this.stopJobAutoRefresh();
				this.pushStreamEvent('open', 'Stream connected');
			},
			onEvent: (event) => {
				if (this.jobStreamAbortController !== controller) return;
				this.applyStreamEvent(event);
			},
			onClose: () => {
				if (this.jobStreamAbortController !== controller) return;
				this.jobStreamAbortController = null;
				this.jobStreamConnected = false;
				this.pushStreamEvent('close', controller.signal.aborted ? 'Stream aborted' : 'Stream closed');
				if (controller.signal.aborted || this.jobStreamTerminal) {
					if (!controller.signal.aborted && this.jobStreamStatus !== 'error') {
						this.jobStreamStatus = 'idle';
					}
					return;
				}
				this.jobStreamStatus = this.jobStreamStatus === 'error' ? 'error' : 'fallback';
				if (this.jobAutoRefreshEnabled) this.startJobAutoRefresh();
				if (this.jobStreamEnabled && this.selectedJobId === targetJobId) {
					this.scheduleJobStreamReconnect();
				}
			}
		}).catch(() => {
			if (this.jobStreamAbortController !== controller) return;
			this.jobStreamAbortController = null;
			if (controller.signal.aborted) return;
			this.jobStreamConnected = false;
			this.jobStreamStatus = 'error';
			this.pushStreamEvent('error', 'Stream fetch failed');
			if (this.jobAutoRefreshEnabled) this.startJobAutoRefresh();
			this.scheduleJobStreamReconnect();
		});
	}

	private restartJobRealtime(): void {
		this.stopJobAutoRefresh();
		if (!this.selectedJobId) {
			this.stopJobStream();
			this.jobStreamStatus = 'idle';
			return;
		}
		if (this.jobStreamEnabled) {
			this.startJobStream();
			return;
		}
		this.stopJobStream();
		if (this.jobAutoRefreshEnabled) this.startJobAutoRefresh();
	}

	public startJobAutoRefreshDomain(): void {
		this.startJobAutoRefresh();
	}

	public stopJobAutoRefreshDomain(): void {
		this.stopJobAutoRefresh();
	}

	public restartJobRealtimeDomain(): void {
		this.restartJobRealtime();
	}

	public stopJobStreamDomain(): void {
		this.stopJobStream();
	}

	public upsertJobSummaryDomain(summary: AgentJobSummary): void {
		this.upsertJobSummary(summary);
	}

	public setJobAutoRefreshEnabled(checked: boolean): void {
		setJobAutoRefreshEnabledDomain(this, checked);
	}

	public setJobAutoRefreshMs(value: number): void {
		setJobAutoRefreshMsDomain(this, value);
	}

	public setJobStreamEnabled(checked: boolean): void {
		setJobStreamEnabledDomain(this, checked);
	}

	public selectJob = (id: string) => {
		selectJobDomain(this, id);
	};

	public cancelSelectedJob = async () => {
		await cancelSelectedJobDomain(this);
	};

	public selectJobHost = (hostId: string) => {
		selectJobHostDomain(this, hostId);
	};

	public setSelectedJobTail = (tail: number) => {
		setSelectedJobTailDomain(this, tail);
	};

	public dispose(): void {
		this.stopJobStream();
		this.stopJobAutoRefresh();
	}

	public initialize = async () => {
		await Promise.all([
			this.refreshHosts(),
			this.refreshCatalog(),
			this.refreshJobs(),
			this.refreshLauncherStatus(),
			this.refreshPluginHealth(),
			this.refreshBlueprints(),
			this.refreshConfig(),
			this.refreshBuildInsights()
		]);
		if (!this.selectedJobId && this.jobRows.length > 0) this.selectedJobId = this.jobRows[0].id;
		if (this.selectedJobId) {
			await this.fetchJobDetail(this.selectedJobId, this.selectedJobTail, true);
		}
		this.restartJobRealtime();
	};

	public refreshCatalog = async () => {
		try {
			this.operationCatalog = await AgentRepo.fetchCatalog();
		} catch {
			this.operationCatalog = [];
		}
	};

	public refreshHosts = async () => {
		try {
			const rows = await AgentRepo.fetchHosts();
			this.hostRows = rows.length > 0 ? rows : [{ id: 'local', name: 'Local', url: appState.agentUrl, enabled: true }];
			if (!this.hostRows.some((row) => row.id === this.selectedJobHostId)) {
				this.selectedJobHostId = 'local';
			}
		} catch {
			this.hostRows = [{ id: 'local', name: 'Local', url: appState.agentUrl, enabled: true }];
			this.selectedJobHostId = 'local';
			appState.addAudit('HOSTS_FETCH_FAILED', 'failure');
		}
	};

	public refreshBlueprints = async () => {
		const cached = this.loadBlueprintCache();
		if (cached && cached.length > 0) this.blueprints = cached;
		try {
			const rows = await AgentRepo.fetchBlueprintList();
			if (rows.length > 0) {
				this.blueprints = rows;
				this.persistBlueprintCache(rows);
				return;
			}
		} catch {
			appState.addAudit('BLUEPRINT_LIST_FETCH_FAILED', 'failure');
		}
		if (this.blueprints.length === 0) this.blueprints = this.fallbackBlueprints;
	};

	public handleExecution = async (id: string) => {
		this.isExecuting = true;
		appState.addAudit(`EXEC_BLUEPRINT_INIT: ${id}`);
		try {
			await AgentRepo.executeBlueprint(id);
			appState.addAudit(`EXEC_SUCCESS: ${id}`, 'success');
		} catch {
			appState.addAudit(`EXEC_FAILURE: ${id}`, 'failure');
		} finally {
			setTimeout(() => (this.isExecuting = false), 350);
		}
	};

	public runOperation = async (action: AgentCatalogAction) => {
		this.operationsBusy = true;
		this.operationMessage = '';
		let confirmationId = '';
		try {
			if (action.risk === 'HIGH') {
				const confirmation = await AgentRepo.requestConfirmation(action.id);
				confirmationId = confirmation?.id ?? '';
			}
			const result = await AgentRepo.runActionAsync({
				action: action.id,
				priority: this.operationPriority,
				confirmationId: confirmationId || undefined
			});
			const jobId = result.job?.id ?? 'n/a';
			this.operationMessage = `${m.control_operation_queued()} #${jobId}`;
			this.selectedJobId = result.job?.id ?? this.selectedJobId;
			appState.addAudit(`CONTROL_OP_ACCEPTED:${action.id}:${jobId}`);
			await this.refreshJobs();
			if (result.job?.id) await this.fetchJobDetail(result.job.id, this.selectedJobTail, true);
		} catch {
			this.operationMessage = m.control_operation_failed();
			appState.addAudit(`CONTROL_OP_FAILED:${action.id}`, 'failure');
		} finally {
			this.operationsBusy = false;
		}
	};

	public requestOperation = (action: AgentCatalogAction) => {
		if (action.risk === 'HIGH') {
			this.pendingAction = action;
			return;
		}
		void this.runOperation(action);
	};

	public cancelPendingAction = () => {
		this.pendingAction = null;
	};

	public confirmPendingAction = () => {
		const action = this.pendingAction;
		this.pendingAction = null;
		if (action) void this.runOperation(action);
	};

	public revertField = (field: ConfigFieldSpec) => {
		if (!this.configSnapshot) return;
		this.setDraftValue(field.path, this.toDraftText(this.configSnapshot.values[field.path]));
	};

	public toDraftText(value: ConfigValuePrimitive | undefined): string {
		return toDraftTextDomain(value);
	}

	public getDraftValue = (path: string) => this.configDraft[path] ?? '';
	public setDraftValue = (path: string, value: string) => {
		this.configDraft = { ...this.configDraft, [path]: value };
	};
	public setDraftBool = (path: string, checked: boolean) => {
		this.setDraftValue(path, checked ? 'true' : 'false');
	};

	public parseDraftValue(field: ConfigFieldSpec): ConfigValuePrimitive {
		return parseDraftValueDomain(this, field);
	}

	public isFieldChanged = (field: ConfigFieldSpec): boolean => {
		return isFieldChangedDomain(this, field);
	};

	public getPendingConfigUpdates(): ConfigUpdateEntry[] {
		return getPendingConfigUpdatesDomain(this);
	}

	public refreshConfig = async () => {
		await refreshConfigDomain(this);
	};

	public refreshJobs = async () => {
		await refreshJobsDomain(this);
	};

	public fetchJobDetail = async (id: string, tail = this.selectedJobTail, silent = false) => {
		await fetchJobDetailDomain(this, id, tail, silent);
	};

	public applyMutationSnapshot(config: AgentConfigPayload | undefined): void {
		applyMutationSnapshotDomain(this, config);
	}

	public applyConfigUpdates = async () => {
		await applyConfigUpdatesDomain(this);
	};

	public cancelJob = async (id: string) => {
		await cancelJobDomain(this, id);
	};

	public applyAutoPreset = async () => {
		await applyAutoPresetDomain(this);
	};

	public applyComposeProfile = async () => {
		await applyComposeProfileDomain(this);
	};

	public refreshBuildInsights = async () => {
		await refreshBuildInsightsDomain(this);
	};

	public refreshComposeRecommendation = async () => {
		await refreshComposeRecommendationDomain(this);
	};

	public refreshOverrideTemplate = async () => {
		await refreshOverrideTemplateDomain(this);
	};

	public exportProfile = async () => {
		await exportProfileDomain(this);
	};

	public importProfile = async () => {
		await importProfileDomain(this);
	};

	public applyDriftFix = async () => {
		await applyDriftFixDomain(this);
	};

	public refreshLauncherStatus = async () => {
		try {
			appState.launcherStatus = await LauncherRepo.fetchAgentStatus();
		} catch {
			appState.launcherStatus = { status: 'offline', pid: null };
		}
	};

	public refreshPluginHealth = async () => {
		try {
			appState.pluginHealth = await AgentRepo.fetchPluginHealth();
		} catch {
			appState.pluginHealth = {};
		}
	};

	public runLauncherAction = async (action: LauncherAction) => {
		this.launcherBusy = true;
		try {
			if (action === 'start') await LauncherRepo.startAgent();
			if (action === 'stop') await LauncherRepo.stopAgent();
			if (action === 'restart') await LauncherRepo.restartAgent();
			appState.addAudit(`LAUNCHER_${action.toUpperCase()}_REQUESTED`);
			await this.refreshLauncherStatus();
		} catch {
			appState.addAudit(`LAUNCHER_${action.toUpperCase()}_FAILED`, 'failure');
		} finally {
			this.launcherBusy = false;
		}
	};
}

export function createControlCenterState(): ControlCenterState {
	return new ControlCenterState();
}
