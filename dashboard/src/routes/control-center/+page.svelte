<script lang="ts">
	import ControlCenterConfirmModal from '$lib/components/control-center/ControlCenterConfirmModal.svelte';
	import ControlCenterHeader from '$lib/components/control-center/ControlCenterHeader.svelte';
	import ControlCenterMissionPanel from '$lib/components/control-center/ControlCenterMissionPanel.svelte';
	import ControlCenterTabRouter from '$lib/components/control-center/ControlCenterTabRouter.svelte';
	import ControlCenterTabs from '$lib/components/control-center/ControlCenterTabs.svelte';
	import { createControlCenterState } from '$lib/components/control-center/control-center.state.svelte';
	import { createAutonomyRunner, type AutoAction } from '$lib/components/control-center/state/autonomy-runner.svelte';
	import type { AgentCatalogAction } from '$lib/types';
	import { m } from '$lib/paraglide/messages';
	import { appState } from '$lib/state.svelte';
	import { onDestroy, onMount } from 'svelte';

	const controller = createControlCenterState();
	const autonomy = createAutonomyRunner();

	type FlowSection = {
		id: string;
		tab: (typeof controller.tabs)[number]['id'];
		title: string;
		detail: string;
		subsections: string[];
	};

	const flowSections = $derived.by((): FlowSection[] => [
		{
			id: 'operations',
			tab: 'operations',
			title: m.cc_lane_operations_title(),
			detail: m.cc_lane_operations_detail(),
			subsections: [m.cc_lane_operations_sub1(), m.cc_lane_operations_sub2(), m.cc_lane_operations_sub3()]
		},
		{
			id: 'build',
			tab: 'build',
			title: m.cc_lane_build_title(),
			detail: m.cc_lane_build_detail(),
			subsections: [m.cc_lane_build_sub1(), m.cc_lane_build_sub2(), m.cc_lane_build_sub3()]
		},
		{
			id: 'config',
			tab: 'config',
			title: m.cc_lane_config_title(),
			detail: m.cc_lane_config_detail(),
			subsections: [m.cc_lane_config_sub1(), m.cc_lane_config_sub2(), m.cc_lane_config_sub3()]
		},
		{
			id: 'plugins',
			tab: 'plugins',
			title: m.cc_lane_runtime_title(),
			detail: m.cc_lane_runtime_detail(),
			subsections: [m.cc_lane_runtime_sub1(), m.cc_lane_runtime_sub2(), m.cc_lane_runtime_sub3()]
		}
	]);

	const glossary = [
		{ term: m.cc_term_compose(), meaning: m.cc_term_compose_meaning() },
		{ term: m.cc_term_drift(), meaning: m.cc_term_drift_meaning() },
		{ term: m.cc_term_preset(), meaning: m.cc_term_preset_meaning() },
		{ term: m.cc_term_stream(), meaning: m.cc_term_stream_meaning() },
		{ term: m.cc_term_circuit(), meaning: m.cc_term_circuit_meaning() }
	];

	const suggestedOperation = $derived.by(() => {
		const rows = controller.operationRows;
		const preferred = ['doctor_fix', 'dashboard_build', 'qemu_smoke', 'quality_gate'];
		for (const id of preferred) {
			const found = rows.find((row) => row.id === id);
			if (found) return found;
		}
		return rows[0] ?? null;
	});

	const hasDriftMismatch = $derived.by(() =>
		Boolean(
			controller.driftReport?.goals.some((goal) => goal.missingCount > 0 || goal.extraCount > 0)
		)
	);

	async function runSuggestedOperation(action: AgentCatalogAction | null): Promise<void> {
		if (!action) return;
		controller.activeTab = 'operations';
		controller.requestOperation(action);
	}

	const autonomyActions = $derived.by((): AutoAction[] => {
		const rows: AutoAction[] = [];

		if (!appState.isConnected) {
			rows.push({
				id: 'recover-connectivity',
				tone: 'error',
				title: m.cc_auto_recover_title(),
				detail: m.cc_auto_recover_detail(),
				cta: m.cc_auto_recover_cta(),
				precheck: () => !appState.isConnected,
				run: async () => {
					await controller.runLauncherAction('restart');
					await controller.refreshLauncherStatus();
					await controller.refreshPluginHealth();
				},
				verify: () => appState.launcherStatus.status !== 'offline'
			});
		}

		rows.push({
			id: 'sync-system',
			tone: 'info',
			title: m.cc_auto_sync_title(),
			detail: m.cc_auto_sync_detail(),
			cta: m.cc_auto_sync_cta(),
			precheck: () => true,
			run: async () => {
				await Promise.all([
					controller.refreshJobs(),
					controller.refreshBuildInsights(),
					controller.refreshPluginHealth(),
					controller.refreshLauncherStatus()
				]);
			},
			verify: () => controller.jobRows.length >= 0
		});

		rows.push({
			id: 'repair-drift',
			tone: hasDriftMismatch ? 'warning' : 'success',
			title: m.cc_auto_drift_title(),
			detail: m.cc_auto_drift_detail(),
			cta: m.cc_auto_drift_cta(),
			precheck: () => true,
			run: async () => {
				controller.activeTab = 'build';
				await controller.refreshBuildInsights();
			},
			verify: () => controller.activeTab === 'build'
		});

		rows.push({
			id: 'queue-best-operation',
			tone: 'info',
			title: m.cc_auto_queue_title(),
			detail: suggestedOperation
				? `${m.cc_auto_queue_best_prefix()} ${suggestedOperation.title} (${suggestedOperation.risk}).`
				: m.cc_auto_queue_none(),
			cta: m.cc_auto_queue_cta(),
			precheck: () => Boolean(suggestedOperation),
			run: async () => {
				await runSuggestedOperation(suggestedOperation);
			},
			verify: () => controller.activeTab === 'operations'
		});

		return rows;
	});

	onMount(() => {
		void controller.initialize();
	});

	onDestroy(() => {
		controller.dispose();
	});
</script>

<div class="space-y-8 lg:space-y-10">
	<ControlCenterMissionPanel
		{flowSections}
		{glossary}
		{autonomyActions}
		autonomyWorkflow={autonomy.workflow}
		autonomyBusyId={autonomy.busyId}
		autonomyMessage={autonomy.message}
		onSelectTab={(tab) => (controller.activeTab = tab as (typeof controller.tabs)[number]['id'])}
		onRunAutonomy={(id) => autonomy.runById(id, autonomyActions)}
		runningLabel={m.cc_running_label()}
		workflowTitle={m.cc_workflow_title()}
	/>

	<ControlCenterHeader
		launcherStatus={appState.launcherStatus}
		liveMode={appState.liveMode}
		streamConnected={appState.streamConnected}
		launcherBusy={controller.launcherBusy}
		latencyMs={appState.latencyMs}
		retryCount={appState.retryCount}
		apiCircuitOpen={appState.apiCircuitOpen}
		connectionQuality={appState.connectionQuality}
		onLauncherAction={controller.runLauncherAction}
	/>

	<ControlCenterTabs
		activeTab={controller.activeTab}
		tabs={controller.tabs}
		onSelect={(tab) => (controller.activeTab = tab)}
	/>

	<ControlCenterTabRouter {controller} />

	<ControlCenterConfirmModal
		pendingAction={controller.pendingAction}
		onCancel={controller.cancelPendingAction}
		onConfirm={controller.confirmPendingAction}
	/>
</div>
