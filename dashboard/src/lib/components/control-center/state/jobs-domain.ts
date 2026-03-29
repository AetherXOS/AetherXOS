import { AgentRepo } from '$lib/api';
import { appState } from '$lib/state.svelte';
import type { ControlCenterState } from '../control-center.state.svelte';

export function setJobAutoRefreshEnabledDomain(state: ControlCenterState, checked: boolean): void {
	state.jobAutoRefreshEnabled = checked;
	if (checked && (!state.jobStreamEnabled || !state.jobStreamConnected)) state.startJobAutoRefreshDomain();
	else state.stopJobAutoRefreshDomain();
}

export function setJobAutoRefreshMsDomain(state: ControlCenterState, value: number): void {
	const ms = Number.isFinite(value) ? value : 5000;
	state.jobAutoRefreshMs = Math.max(1500, Math.min(30000, ms));
	if (state.jobAutoRefreshEnabled && (!state.jobStreamEnabled || !state.jobStreamConnected)) {
		state.startJobAutoRefreshDomain();
	}
}

export function setJobStreamEnabledDomain(state: ControlCenterState, checked: boolean): void {
	state.jobStreamEnabled = checked;
	state.restartJobRealtimeDomain();
}

export function selectJobDomain(state: ControlCenterState, id: string): void {
	state.selectedJobId = id;
	state.streamEvents = [];
	state.streamReconnectCount = 0;
	if (id) void state.fetchJobDetail(id, state.selectedJobTail, true);
	state.restartJobRealtimeDomain();
}

export async function cancelSelectedJobDomain(state: ControlCenterState): Promise<void> {
	if (!state.selectedJobId) return;
	try {
		await AgentRepo.cancelJob(state.selectedJobId, state.selectedJobHostId);
		appState.addAudit(`JOB_CANCELLED:${state.selectedJobId}`, 'success');
		await state.refreshJobs();
		if (state.selectedJobId) await state.fetchJobDetail(state.selectedJobId, state.selectedJobTail, true);
	} catch {
		appState.addAudit(`JOB_CANCEL_FAILED:${state.selectedJobId}`, 'failure');
	}
}

export function selectJobHostDomain(state: ControlCenterState, hostId: string): void {
	const nextHost = hostId?.trim() || 'local';
	if (nextHost === state.selectedJobHostId) return;
	state.selectedJobHostId = nextHost;
	state.selectedJobId = '';
	state.selectedJobDetail = null;
	void state.refreshJobs();
}

export function setSelectedJobTailDomain(state: ControlCenterState, tail: number): void {
	state.selectedJobTail = Math.max(20, Math.min(4000, Number.isFinite(tail) ? tail : 400));
	if (state.selectedJobId) void state.fetchJobDetail(state.selectedJobId, state.selectedJobTail, true);
	state.restartJobRealtimeDomain();
}

export async function refreshJobsDomain(state: ControlCenterState): Promise<void> {
	state.jobsBusy = true;
	try {
		state.jobRows = await AgentRepo.fetchJobs(state.selectedJobHostId);
		if (!state.selectedJobId && state.jobRows.length > 0) state.selectedJobId = state.jobRows[0].id;
		if (state.selectedJobId && !state.jobRows.some((row) => row.id === state.selectedJobId)) {
			state.selectedJobId = state.jobRows[0]?.id ?? '';
			if (!state.selectedJobId) state.selectedJobDetail = null;
		}
		if (state.selectedJobId) {
			await state.fetchJobDetail(state.selectedJobId, state.selectedJobTail, true);
		}
		state.restartJobRealtimeDomain();
	} catch {
		state.jobRows = [];
		state.selectedJobDetail = null;
		state.stopJobStreamDomain();
		appState.addAudit('JOBS_FETCH_FAILED', 'failure');
	} finally {
		state.jobsBusy = false;
	}
}

export async function fetchJobDetailDomain(
	state: ControlCenterState,
	id: string,
	tail = state.selectedJobTail,
	silent = false
): Promise<void> {
	if (!silent) {
		state.jobDetailBusy = true;
		state.buildMessage = '';
	}
	state.selectedJobId = id;
	state.selectedJobTail = Math.max(20, Math.min(4000, tail));
	try {
		state.selectedJobDetail = await AgentRepo.fetchJob(id, state.selectedJobTail, state.selectedJobHostId);
		state.upsertJobSummaryDomain(state.selectedJobDetail.job);
	} catch {
		if (!silent) {
			state.buildMessage = `Job output fetch failed: ${id}`;
			appState.addAudit(`JOB_DETAIL_FAILED:${id}`, 'failure');
		}
	} finally {
		if (!silent) state.jobDetailBusy = false;
	}
}

export async function cancelJobDomain(state: ControlCenterState, id: string): Promise<void> {
	state.jobsBusy = true;
	try {
		await AgentRepo.cancelJob(id, state.selectedJobHostId);
		state.operationMessage = `Job cancelled: ${id} @ ${state.selectedJobHostId}`;
		appState.addAudit(`JOB_CANCELLED:${id}`);
		await state.refreshJobs();
		if (state.selectedJobId === id) await state.fetchJobDetail(id, state.selectedJobTail, true);
	} catch {
		state.operationMessage = `Job cancel failed: ${id} @ ${state.selectedJobHostId}`;
		appState.addAudit(`JOB_CANCEL_FAILED:${id}`, 'failure');
	} finally {
		state.jobsBusy = false;
	}
}
