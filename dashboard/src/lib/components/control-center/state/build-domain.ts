import { AgentRepo } from '$lib/api';
import { appState } from '$lib/state.svelte';
import type { ControlCenterState } from '../control-center.state.svelte';

export async function refreshBuildInsightsDomain(state: ControlCenterState): Promise<void> {
	state.buildBusy = true;
	state.buildMessage = '';
	try {
		const [recommendation, drift, template, compliance, crash] = await Promise.all([
			AgentRepo.fetchComposeRecommendation({ goal: state.composeGoal, minimal: state.composeMinimal }),
			AgentRepo.fetchConfigDrift(),
			AgentRepo.fetchConfigOverrideTemplate(state.overrideTemplateMode),
			AgentRepo.fetchComplianceReport(),
			AgentRepo.fetchCrashSummary()
		]);
		state.composeRecommendation = recommendation;
		state.driftReport = drift;
		state.overrideTemplate = template;
		state.complianceReport = compliance;
		state.crashSummary = crash;
	} catch {
		state.buildMessage = 'Build insights refresh failed.';
		appState.addAudit('BUILD_INSIGHTS_FAILED', 'failure');
	} finally {
		state.buildBusy = false;
	}
}

export async function refreshComposeRecommendationDomain(state: ControlCenterState): Promise<void> {
	state.buildBusy = true;
	try {
		state.composeRecommendation = await AgentRepo.fetchComposeRecommendation({
			goal: state.composeGoal,
			minimal: state.composeMinimal
		});
	} catch {
		state.buildMessage = 'Build recommendation fetch failed.';
	} finally {
		state.buildBusy = false;
	}
}

export async function refreshOverrideTemplateDomain(state: ControlCenterState): Promise<void> {
	state.buildBusy = true;
	try {
		state.overrideTemplate = await AgentRepo.fetchConfigOverrideTemplate(state.overrideTemplateMode);
	} catch {
		state.buildMessage = 'Override template export failed.';
	} finally {
		state.buildBusy = false;
	}
}

export async function exportProfileDomain(state: ControlCenterState): Promise<void> {
	state.buildBusy = true;
	try {
		state.exportedProfile = await AgentRepo.exportConfigProfile(state.exportProfileName.trim() || 'default');
		state.buildMessage = `Exported profile: ${state.exportedProfile.profileName}`;
	} catch {
		state.buildMessage = 'Config profile export failed.';
	} finally {
		state.buildBusy = false;
	}
}

export async function importProfileDomain(state: ControlCenterState): Promise<void> {
	const raw = state.importProfileText.trim();
	if (!raw) {
		state.buildMessage = 'Paste a config profile JSON payload first.';
		return;
	}
	state.buildBusy = true;
	try {
		const parsed = JSON.parse(raw);
		const result = await AgentRepo.importConfigProfile(parsed);
		state.applyMutationSnapshot(result.config);
		state.buildMessage = `Imported ${result.applied.length} values from profile.`;
		await state.refreshBuildInsights();
	} catch {
		state.buildMessage = 'Config profile import failed.';
		appState.addAudit('CONFIG_IMPORT_FAILED', 'failure');
	} finally {
		state.buildBusy = false;
	}
}

export async function applyDriftFixDomain(state: ControlCenterState): Promise<void> {
	state.buildBusy = true;
	try {
		const result = await AgentRepo.applyConfigDrift({
			goal: state.composeGoal,
			mode: state.driftApplyMode,
			minimal: state.composeMinimal,
			noDefaultFeatures: state.composeRecommendation?.noDefaultFeatures
		});
		state.applyMutationSnapshot(result.config);
		state.driftReport = result.drift ?? state.driftReport;
		state.composeRecommendation = result.recommendation ?? state.composeRecommendation;
		state.buildMessage = `Applied drift repair for ${state.composeGoal} (${state.driftApplyMode}).`;
		await state.refreshBuildInsights();
	} catch {
		state.buildMessage = 'Config drift apply failed.';
		appState.addAudit('CONFIG_DRIFT_APPLY_FAILED', 'failure');
	} finally {
		state.buildBusy = false;
	}
}
