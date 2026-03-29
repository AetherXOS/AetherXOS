<script lang="ts">
	import { page } from '$app/stores';
	import { LauncherRepo } from '$lib/api';
	import { orchestrator } from '$lib/services/orchestrator';
	import { appState } from '$lib/state.svelte';
	import {
		getTaskMap,
		getWorkflowProgressPercent,
		markRouteVisited,
		setTaskDone
	} from '$lib/services/workflow-progress';
	import { AlertTriangle, CheckCircle2, Compass, RefreshCw, ShieldAlert, Wrench } from 'lucide-svelte';

	let actionBusy = $state(false);
	let taskMap = $state<Record<string, boolean>>({});

	const crumb = $derived.by(() => {
		const path = $page.url.pathname;
		if (path.startsWith('/settings')) return 'Setup > Settings';
		if (path.startsWith('/executive')) return 'Monitor > Executive';
		if (path.startsWith('/operations')) return 'Respond > Operations';
		if (path.startsWith('/control-center')) return 'Operate > Control Center';
		if (path.startsWith('/deep-debug')) return 'Diagnose > Deep Debug';
		return 'Home';
	});

	const nowLabel = $derived.by(() => {
		if (!appState.isConnected) return 'Agent offline';
		if (appState.criticalIncidentCount > 0) return `${appState.criticalIncidentCount} critical incident(s)`;
		if (appState.apiCircuitOpen) return 'API circuit protection active';
		return 'Platform stable';
	});

	const nextLabel = $derived.by(() => {
		if (!appState.isConnected) return 'Recover connectivity';
		if (appState.criticalIncidentCount > 0) return 'Open Operations and triage critical incidents';
		if (!appState.streamConnected) return 'Re-enable real-time stream';
		return 'Run preventive checks in Control Center';
	});

	const blockerLabel = $derived.by(() => {
		if (!appState.isConnected) return 'No live metrics or incident stream';
		if (appState.apiCircuitOpen) return 'Request flow throttled by circuit breaker';
		if (!appState.streamConnected) return 'Live stream disconnected; dashboard may lag';
		return 'No active blocker';
	});

	const progress = $derived(getWorkflowProgressPercent());

	$effect(() => {
		markRouteVisited($page.url.pathname);
		taskMap = getTaskMap();
	});

	async function runRecoveryPack() {
		actionBusy = true;
		try {
			await LauncherRepo.restartAgent().catch(() => undefined);
			await orchestrator.sync(true);
			setTaskDone('recovery-pack', true);
			taskMap = getTaskMap();
		} finally {
			actionBusy = false;
		}
	}
</script>

<section class="rounded-2xl border border-white/8 bg-base-200/75 p-4 sm:p-5">
	<div class="grid grid-cols-1 gap-3 xl:grid-cols-[1fr_1fr_1fr_auto]">
		<div class="rounded-xl border border-white/10 bg-base-100/40 p-3 text-xs">
			<div class="mb-1 flex items-center gap-2 font-black uppercase tracking-wide opacity-60"><Compass size={12} /> Now</div>
			<div class="leading-relaxed">{nowLabel}</div>
		</div>
		<div class="rounded-xl border border-white/10 bg-base-100/40 p-3 text-xs">
			<div class="mb-1 flex items-center gap-2 font-black uppercase tracking-wide opacity-60"><Wrench size={12} /> Next</div>
			<div class="leading-relaxed">{nextLabel}</div>
		</div>
		<div class="rounded-xl border border-white/10 bg-base-100/40 p-3 text-xs">
			<div class="mb-1 flex items-center gap-2 font-black uppercase tracking-wide opacity-60"><ShieldAlert size={12} /> Blocker</div>
			<div class="leading-relaxed">{blockerLabel}</div>
		</div>
		<div class="flex flex-col gap-2 xl:min-w-62">
			<button class="btn btn-sm btn-primary gap-2" onclick={runRecoveryPack} disabled={actionBusy}>
				{#if actionBusy}
					<RefreshCw size={14} class="animate-spin" />
					Recovering...
				{:else}
					<AlertTriangle size={14} />
					One-click recovery
				{/if}
			</button>
			<div class="rounded-xl border border-white/10 bg-base-100/40 px-3 py-2 text-[11px]">
				<div class="mb-1 flex items-center justify-between font-black uppercase tracking-wide opacity-60">
					<span>Workflow</span>
					<span>{progress}%</span>
				</div>
				<div class="h-1.5 overflow-hidden rounded-full bg-base-300">
					<div class="h-full rounded-full bg-primary transition-all duration-500" style={`width:${progress}%`}></div>
				</div>
				<div class="mt-1 flex items-center gap-1 opacity-60">
					{#if taskMap['recovery-pack']}
						<CheckCircle2 size={11} class="text-success" />
						Recovery pack used
					{:else}
						<AlertTriangle size={11} class="text-warning" />
						Recovery pack not used yet
					{/if}
				</div>
			</div>
		</div>
	</div>
	<div class="mt-3 text-[11px] opacity-45">Path: {crumb}</div>
</section>
