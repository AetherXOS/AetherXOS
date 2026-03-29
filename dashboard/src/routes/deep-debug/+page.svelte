<script lang="ts">
	import { resolve } from '$app/paths';
	import DeepDebugArchitectureSection from '$lib/components/deep-debug/DeepDebugArchitectureSection.svelte';
	import DeepDebugLogViewer from '$lib/components/deep-debug/DeepDebugLogViewer.svelte';
	import { m } from '$lib/paraglide/messages';
	import { appState } from '$lib/state.svelte';
	import {
		Bug,
		Compass,
		RefreshCw,
		Wrench
	} from 'lucide-svelte';

	type LevelFilter = 'all' | 'audit' | 'error' | 'critical';

	let search = $state('');
	let level = $state<LevelFilter>('all');

	const debugLines = $derived.by(() => {
		const lines = [
			...appState.auditLogs.slice(0, 120).map((log) => ({
				id: `audit-${log.id}`,
				ts: Date.parse(log.timestamp) || Date.now(),
				time: new Date(log.timestamp).toLocaleTimeString(),
				kind: 'audit' as const,
				level: log.status === 'failure' ? 'error' as const : 'audit' as const,
				msg: `${log.action} [${log.operator}]`
			})),
			...appState.incidents.slice(0, 120).map((incident) => ({
				id: `incident-${incident.id}`,
				ts: Date.parse(incident.timestamp) || Date.now(),
				time: new Date(incident.timestamp).toLocaleTimeString(),
				kind: 'incident' as const,
				level: incident.severity === 'critical' ? 'critical' as const : 'error' as const,
				msg: `${incident.type}: ${incident.message}`
			}))
		];

		const q = search.trim().toLowerCase();
		return lines
			.filter((line) => {
				if (level !== 'all' && line.level !== level) return false;
				if (!q) return true;
				return line.msg.toLowerCase().includes(q);
			})
			.sort((a, b) => b.ts - a.ts)
			.slice(0, 220);
	});

	const debugLayers = [
		{
			id: 'capture',
			title: m.debug_layer_capture_title(),
			detail: m.debug_layer_capture_detail(),
			subsections: [m.debug_layer_capture_sub1(), m.debug_layer_capture_sub2()]
		},
		{
			id: 'filter',
			title: m.debug_layer_filter_title(),
			detail: m.debug_layer_filter_detail(),
			subsections: [m.debug_layer_filter_sub1(), m.debug_layer_filter_sub2()]
		},
		{
			id: 'resolve',
			title: m.debug_layer_resolve_title(),
			detail: m.debug_layer_resolve_detail(),
			subsections: [m.debug_layer_resolve_sub1(), m.debug_layer_resolve_sub2()]
		}
	];

	const debugDefinitions = [
		{ term: m.debug_term_audit(), meaning: m.debug_term_audit_meaning() },
		{ term: m.debug_term_incident(), meaning: m.debug_term_incident_meaning() },
		{ term: m.debug_term_critical(), meaning: m.debug_term_critical_meaning() }
	];

	function focusCritical() {
		search = '';
		level = 'critical';
	}

	function focusErrors() {
		search = '';
		level = 'error';
	}

	function resetDebugFilters() {
		search = '';
		level = 'all';
	}
</script>

<div class="space-y-10">
	<h1 class="text-7xl font-black uppercase italic">{m.debug_title()}</h1>

	<DeepDebugArchitectureSection
		{debugLayers}
		{debugDefinitions}
		onFocusCritical={focusCritical}
		onFocusErrors={focusErrors}
		onResetFilters={resetDebugFilters}
	/>

	<div class="grid grid-cols-1 gap-3 md:grid-cols-3">
		<input
			class="input input-sm bg-base-200 md:col-span-2"
			bind:value={search}
			placeholder={m.debug_search_placeholder()}
		/>
		<select class="select select-sm bg-base-200" bind:value={level}>
			<option value="all">{m.debug_level_all()}</option>
			<option value="audit">{m.debug_level_audit()}</option>
			<option value="error">{m.debug_level_error()}</option>
			<option value="critical">{m.debug_level_critical()}</option>
		</select>
	</div>

	<div class="flex flex-wrap gap-2 text-xs">
		<button class="btn btn-sm btn-outline gap-2" onclick={focusCritical}>
			<Bug size={14} />
			{m.debug_quick_critical()}
		</button>
		<button class="btn btn-sm btn-outline gap-2" onclick={focusErrors}>
			<Wrench size={14} />
			{m.debug_quick_errors()}
		</button>
		<button class="btn btn-sm btn-outline gap-2" onclick={resetDebugFilters}>
			<RefreshCw size={14} />
			{m.debug_quick_reset()}
		</button>
		<a href={resolve('/operations')} class="btn btn-sm btn-ghost gap-2">
			<Compass size={14} />
			{m.debug_quick_ops()}
		</a>
	</div>

	<DeepDebugLogViewer
		lines={debugLines}
		isConnected={appState.isConnected}
		hasFilter={Boolean(search || level !== 'all')}
	/>
</div>

