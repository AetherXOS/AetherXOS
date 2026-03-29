<script lang="ts">
	import { resolve } from '$app/paths';
	import OperationsArchitecturePanel from '$lib/components/operations/OperationsArchitecturePanel.svelte';
	import OperationsIncidentFeedPanel from '$lib/components/operations/OperationsIncidentFeedPanel.svelte';
	import OperationsRunbookPanel from '$lib/components/operations/OperationsRunbookPanel.svelte';
	import { m } from '$lib/paraglide/messages';
	import { orchestrator } from '$lib/services/orchestrator';
	import { appState } from '$lib/state.svelte';
	import type { Incident } from '$lib/types';
	import { RefreshCw } from 'lucide-svelte';

	let isRefreshing = $state(false);
	let severityFilter = $state<'all' | Incident['severity']>('all');
	let statusFilter = $state<'all' | Incident['status']>('all');
	let query = $state('');

	const nodeRows = $derived.by(() => {
		const byNode: Record<
			string,
			{ id: string; open: number; investigating: number; resolved: number; critical: number }
		> = {};

		for (const incident of appState.incidents) {
			const key = incident.nodeId || 'local';
			const row = byNode[key] ?? {
				id: key,
				open: 0,
				investigating: 0,
				resolved: 0,
				critical: 0
			};
			if (incident.status === 'open') row.open += 1;
			if (incident.status === 'investigating') row.investigating += 1;
			if (incident.status === 'resolved') row.resolved += 1;
			if (incident.severity === 'critical') row.critical += 1;
			byNode[key] = row;
		}

		if (Object.keys(byNode).length === 0) {
			byNode.local = { id: 'local', open: 0, investigating: 0, resolved: 0, critical: 0 };
		}

		return Object.values(byNode);
	});

	const filteredIncidents = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return appState.incidents
			.filter((incident) => {
				if (severityFilter !== 'all' && incident.severity !== severityFilter) return false;
				if (statusFilter !== 'all' && incident.status !== statusFilter) return false;
				if (!q) return incident.status !== 'resolved';
				const haystack = `${incident.nodeId} ${incident.type} ${incident.message}`.toLowerCase();
				return haystack.includes(q);
			})
			.slice(0, 24);
	});

	const runbook = $derived.by(() => {
		const openCount = appState.incidents.filter((i) => i.status === 'open').length;
		const investigatingCount = appState.incidents.filter((i) => i.status === 'investigating').length;
		const resolvedCount = appState.incidents.filter((i) => i.status === 'resolved').length;
		return {
			openCount,
			investigatingCount,
			resolvedCount,
			offline: !appState.isConnected,
			hasActive: openCount + investigatingCount > 0
		};
	});

	const activeFilterCount = $derived.by(() => {
		let n = 0;
		if (severityFilter !== 'all') n += 1;
		if (statusFilter !== 'all') n += 1;
		if (query.trim().length > 0) n += 1;
		return n;
	});

	const operationsLayers = [
		{
			id: 'detect',
			title: m.ops_layer_detect_title(),
			detail: m.ops_layer_detect_detail(),
			subsections: [m.ops_layer_detect_sub1(), m.ops_layer_detect_sub2()]
		},
		{
			id: 'triage',
			title: m.ops_layer_triage_title(),
			detail: m.ops_layer_triage_detail(),
			subsections: [m.ops_layer_triage_sub1(), m.ops_layer_triage_sub2()]
		},
		{
			id: 'resolve',
			title: m.ops_layer_resolve_title(),
			detail: m.ops_layer_resolve_detail(),
			subsections: [m.ops_layer_resolve_sub1(), m.ops_layer_resolve_sub2()]
		}
	];

	const operationsDefinitions = [
		{ term: m.ops_term_open(), meaning: m.ops_term_open_meaning() },
		{ term: m.ops_term_investigating(), meaning: m.ops_term_investigating_meaning() },
		{ term: m.ops_term_resolved(), meaning: m.ops_term_resolved_meaning() }
	];

	async function refreshOperations() {
		isRefreshing = true;
		try {
			await orchestrator.sync(true);
		} finally {
			isRefreshing = false;
		}
	}

	function clearFilters() {
		severityFilter = 'all';
		statusFilter = 'all';
		query = '';
	}

	function focusCriticalNow() {
		severityFilter = 'critical';
		statusFilter = 'open';
		query = '';
	}

	function focusInvestigating() {
		severityFilter = 'all';
		statusFilter = 'investigating';
		query = '';
	}
</script>

<div class="space-y-8 lg:space-y-10">
	<header class="flex flex-col gap-6 md:flex-row md:items-center md:justify-between">
		<h1 class="text-4xl font-black italic sm:text-5xl lg:text-6xl">{m.operations_title()}</h1>
		<button class="btn btn-outline" onclick={refreshOperations} disabled={isRefreshing}>
			<RefreshCw size={16} class={isRefreshing ? 'animate-spin' : ''} />
			{isRefreshing ? m.operations_refreshing() : m.operations_refresh_now()}
		</button>
	</header>

	<OperationsRunbookPanel
		runbook={runbook}
		isRefreshing={isRefreshing}
		labels={{
			badgeOffline: m.ops_badge_offline(),
			badgeAttention: m.ops_badge_attention(),
			badgeStable: m.ops_badge_stable(),
			detectTitle: m.ops_detect_title(),
			detectDetail: m.ops_detect_detail(),
			detectOfflineNote: m.ops_detect_offline_note(),
			detectOnlineNote: m.ops_detect_online_note(),
			triageTitle: m.ops_triage_title(),
			triageDetail: m.ops_triage_detail(),
			resolveTitle: m.ops_resolve_title(),
			resolveDetail: m.ops_resolve_detail(),
			configureConnection: m.ops_configure_connection(),
			refreshFeed: m.ops_refresh_feed(),
			focusCritical: m.ops_focus_critical(),
			focusInvestigating: m.ops_focus_investigating()
		}}
		onRefresh={refreshOperations}
		onFocusCritical={focusCriticalNow}
		onFocusInvestigating={focusInvestigating}
	/>

	<OperationsArchitecturePanel
		layers={operationsLayers}
		definitions={operationsDefinitions}
		offline={runbook.offline}
		isRefreshing={isRefreshing}
		labels={{
			title: m.ops_architecture_title(),
			subtitle: m.ops_architecture_subtitle(),
			layersLabel: m.ops_layers_label(),
			autonomyTitle: m.ops_autonomy_title(),
			autoCriticalTitle: m.ops_auto_critical_title(),
			autoCriticalDetail: m.ops_auto_critical_detail(),
			autoRefreshTitle: m.ops_auto_refresh_title(),
			autoRefreshDetail: m.ops_auto_refresh_detail(),
			definitionsTitle: m.ops_definitions_title()
		}}
		onFocusCritical={focusCriticalNow}
		onRefresh={refreshOperations}
	/>

	<div class="grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-4">
		{#each nodeRows as node (node.id)}
			<div class="card bg-base-200 space-y-4 border border-white/5 p-8">
				<h3 class="text-xl font-black">{node.id}</h3>
				<div
					class="badge {node.critical > 0
						? 'badge-error'
						: node.open > 0
							? 'badge-warning'
							: 'badge-success'}"
				>
					{node.critical > 0
						? m.operations_status_critical()
						: node.open > 0
							? m.operations_status_degraded()
							: m.operations_status_online()}
				</div>
				<div class="space-y-1 text-xs opacity-60">
					<div>{m.operations_open()}: {node.open}</div>
					<div>{m.operations_investigating()}: {node.investigating}</div>
					<div>{m.operations_resolved()}: {node.resolved}</div>
				</div>
			</div>
		{/each}
	</div>

	<OperationsIncidentFeedPanel
		isConnected={appState.isConnected}
		incidents={appState.incidents}
		filteredIncidents={filteredIncidents}
		severityFilter={severityFilter}
		statusFilter={statusFilter}
		query={query}
		activeFilterCount={activeFilterCount}
		labels={{
			title: m.operations_live_incident_feed(),
			filtersTitle: m.operations_filters_title(),
			searchPlaceholder: m.operations_search_placeholder(),
			severityAll: m.operations_severity_all(),
			statusCritical: m.operations_status_critical(),
			statusHigh: m.operations_status_high(),
			statusMedium: m.operations_status_medium(),
			statusLow: m.operations_status_low(),
			statusAll: m.operations_status_all(),
			statusOpen: m.operations_open(),
			statusInvestigating: m.operations_investigating(),
			statusResolved: m.operations_resolved(),
			clearFilters: m.operations_clear_filters(),
			emptyFiltered: m.operations_empty_filtered(),
			emptyDefault: m.operations_no_active_incidents()
		}}
		onQueryChange={(value) => (query = value)}
		onSeverityChange={(value) => (severityFilter = value)}
		onStatusChange={(value) => (statusFilter = value)}
		onClearFilters={clearFilters}
	/>
</div>
