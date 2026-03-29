import { AgentRepo } from '$lib/api';
import { appState } from '$lib/state.svelte';
import type {
	AgentConfigPayload,
	ConfigFieldSpec,
	ConfigUpdateEntry,
	ConfigValuePrimitive
} from '$lib/types';
import type { ControlCenterState } from '../control-center.state.svelte';

export function toDraftTextDomain(value: ConfigValuePrimitive | undefined): string {
	if (typeof value === 'boolean') return value ? 'true' : 'false';
	if (typeof value === 'number') return String(value);
	if (typeof value === 'string') return value;
	return '';
}

export function parseDraftValueDomain(
	state: ControlCenterState,
	field: ConfigFieldSpec
): ConfigValuePrimitive {
	const raw = state.getDraftValue(field.path);
	if (field.type === 'bool') return raw === 'true';
	if (field.type === 'int') {
		const parsed = Number.parseInt(raw, 10);
		return Number.isFinite(parsed) ? parsed : 0;
	}
	if (field.type === 'float') {
		const parsed = Number.parseFloat(raw);
		return Number.isFinite(parsed) ? parsed : 0;
	}
	return raw;
}

export function isFieldChangedDomain(state: ControlCenterState, field: ConfigFieldSpec): boolean {
	if (!state.configSnapshot) return false;
	const current = state.configSnapshot.values[field.path];
	const next = parseDraftValueDomain(state, field);
	return String(current ?? '') !== String(next ?? '');
}

export function getPendingConfigUpdatesDomain(state: ControlCenterState): ConfigUpdateEntry[] {
	if (!state.configSnapshot) return [];
	const updates: ConfigUpdateEntry[] = [];
	for (const field of state.configSnapshot.fields) {
		if (field.readonly || !state.isFieldChanged(field)) continue;
		updates.push({ path: field.path, value: state.parseDraftValue(field) });
	}
	return updates;
}

export async function refreshConfigDomain(state: ControlCenterState): Promise<void> {
	state.configBusy = true;
	state.configMessage = '';
	try {
		const payload = await AgentRepo.fetchConfig();
		state.applyMutationSnapshot(payload);
	} catch {
		state.configMessage = 'Config fetch failed.';
		appState.addAudit('CONFIG_FETCH_FAILED', 'failure');
	} finally {
		state.configBusy = false;
	}
}

export function applyMutationSnapshotDomain(
	state: ControlCenterState,
	config: AgentConfigPayload | undefined
): void {
	if (!config) return;
	state.configSnapshot = config;
	const nextDraft: Record<string, string> = {};
	for (const field of config.fields) {
		nextDraft[field.path] = state.toDraftText(config.values[field.path]);
	}
	state.configDraft = nextDraft;
}

export async function applyConfigUpdatesDomain(state: ControlCenterState): Promise<void> {
	const updates = state.getPendingConfigUpdates();
	if (updates.length === 0) {
		state.configMessage = 'No config changes to apply.';
		return;
	}
	state.configBusy = true;
	state.configMessage = '';
	try {
		const result = await AgentRepo.updateConfig(updates);
		state.applyMutationSnapshot(result.config);
		state.configMessage = `Applied ${result.applied.length} config updates.`;
		appState.addAudit(`CONFIG_UPDATED:${result.applied.length}`);
		await state.refreshBuildInsights();
	} catch {
		state.configMessage = 'Config update failed.';
		appState.addAudit('CONFIG_UPDATE_FAILED', 'failure');
	} finally {
		state.configBusy = false;
	}
}

export async function applyAutoPresetDomain(state: ControlCenterState): Promise<void> {
	state.configBusy = true;
	state.configMessage = '';
	try {
		const result = await AgentRepo.applyAutoPreset(state.autoPresetMode);
		state.applyMutationSnapshot(result.config);
		state.configMessage = `Auto preset applied: ${state.autoPresetMode}.`;
		appState.addAudit(`CONFIG_AUTO_PRESET:${state.autoPresetMode}`);
		await state.refreshBuildInsights();
	} catch {
		state.configMessage = 'Auto preset apply failed.';
		appState.addAudit('CONFIG_AUTO_PRESET_FAILED', 'failure');
	} finally {
		state.configBusy = false;
	}
}

export async function applyComposeProfileDomain(state: ControlCenterState): Promise<void> {
	state.configBusy = true;
	state.configMessage = '';
	try {
		const result = await AgentRepo.applyComposeProfile({
			goal: state.composeGoal,
			minimal: state.composeMinimal,
			noDefaultFeatures: state.composeRecommendation?.noDefaultFeatures
		});
		state.applyMutationSnapshot(result.config);
		state.configMessage = `Build feature compose applied: ${state.composeGoal}.`;
		appState.addAudit(`CONFIG_COMPOSE_APPLIED:${state.composeGoal}`);
		await state.refreshBuildInsights();
	} catch {
		state.configMessage = 'Build feature compose apply failed.';
		appState.addAudit('CONFIG_COMPOSE_FAILED', 'failure');
	} finally {
		state.configBusy = false;
	}
}
