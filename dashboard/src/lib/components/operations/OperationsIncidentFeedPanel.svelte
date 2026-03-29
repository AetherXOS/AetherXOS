<script lang="ts">
	import { resolve } from '$app/paths';
	import type { Incident } from '$lib/types';
	import { ShieldAlert, Unplug } from 'lucide-svelte';

	type SortMode = 'newest' | 'severity';

	interface Props {
		isConnected: boolean;
		incidents: Incident[];
		filteredIncidents: Incident[];
		severityFilter: 'all' | Incident['severity'];
		statusFilter: 'all' | Incident['status'];
		query: string;
		activeFilterCount: number;
		labels: {
			title: string;
			filtersTitle: string;
			searchPlaceholder: string;
			severityAll: string;
			statusCritical: string;
			statusHigh: string;
			statusMedium: string;
			statusLow: string;
			statusAll: string;
			statusOpen: string;
			statusInvestigating: string;
			statusResolved: string;
			clearFilters: string;
			emptyFiltered: string;
			emptyDefault: string;
		};
		onQueryChange: (value: string) => void;
		onSeverityChange: (value: 'all' | Incident['severity']) => void;
		onStatusChange: (value: 'all' | Incident['status']) => void;
		onClearFilters: () => void;
	}

	let {
		isConnected,
		incidents,
		filteredIncidents,
		severityFilter,
		statusFilter,
		query,
		activeFilterCount,
		labels,
		onQueryChange,
		onSeverityChange,
		onStatusChange,
		onClearFilters
	}: Props = $props();

	let sortMode = $state<SortMode>('newest');

	function severityScore(value: Incident['severity']): number {
		if (value === 'critical') return 4;
		if (value === 'high') return 3;
		if (value === 'medium') return 2;
		return 1;
	}

	const displayedIncidents = $derived.by(() => {
		const rows = [...filteredIncidents];
		if (sortMode === 'severity') {
			rows.sort((a, b) => {
				const bySeverity = severityScore(b.severity) - severityScore(a.severity);
				if (bySeverity !== 0) return bySeverity;
				return Date.parse(b.timestamp) - Date.parse(a.timestamp);
			});
			return rows;
		}
		rows.sort((a, b) => Date.parse(b.timestamp) - Date.parse(a.timestamp));
		return rows;
	});

	function formatTime(ts: string): string {
		const ms = Date.parse(ts);
		if (Number.isNaN(ms)) return ts;
		return new Intl.DateTimeFormat('en', {
			hour12: false,
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		}).format(ms);
	}
</script>

<section class="card bg-base-200 space-y-6 border border-white/5 p-8">
	<div class="flex items-center gap-3 text-sm font-black tracking-widest uppercase opacity-60">
		<ShieldAlert size={16} />
		{labels.title}
	</div>

	<div class="grid grid-cols-1 gap-3 rounded-2xl border border-white/10 bg-base-100/40 p-4 lg:grid-cols-4">
		<div class="text-[10px] font-black tracking-widest uppercase opacity-40 lg:col-span-4 flex items-center justify-between">
			<span>{labels.filtersTitle}</span>
			<span class="badge badge-ghost border-white/10 text-[10px]">active: {activeFilterCount}</span>
		</div>
		<input
			class="input input-sm bg-base-200"
			value={query}
			oninput={(event) => onQueryChange((event.currentTarget as HTMLInputElement).value)}
			placeholder={labels.searchPlaceholder}
		/>
		<select class="select select-sm bg-base-200" value={severityFilter} onchange={(event) => onSeverityChange((event.currentTarget as HTMLSelectElement).value as 'all' | Incident['severity'])}>
			<option value="all">{labels.severityAll}</option>
			<option value="critical">{labels.statusCritical}</option>
			<option value="high">{labels.statusHigh}</option>
			<option value="medium">{labels.statusMedium}</option>
			<option value="low">{labels.statusLow}</option>
		</select>
		<select class="select select-sm bg-base-200" value={statusFilter} onchange={(event) => onStatusChange((event.currentTarget as HTMLSelectElement).value as 'all' | Incident['status'])}>
			<option value="all">{labels.statusAll}</option>
			<option value="open">{labels.statusOpen}</option>
			<option value="investigating">{labels.statusInvestigating}</option>
			<option value="resolved">{labels.statusResolved}</option>
		</select>
		<button class="btn btn-sm btn-outline" onclick={onClearFilters}>
			{labels.clearFilters}
		</button>
	</div>

	<div class="flex flex-wrap items-center gap-2 text-[11px] font-bold uppercase tracking-wide opacity-60">
		<span class="badge badge-ghost border-white/10">total {incidents.length}</span>
		<span class="badge badge-ghost border-white/10">shown {filteredIncidents.length}</span>
		<span class="opacity-50">sort</span>
		<button
			type="button"
			class="btn btn-xs {sortMode === 'newest' ? 'btn-primary' : 'btn-ghost'}"
			onclick={() => (sortMode = 'newest')}
		>
			newest
		</button>
		<button
			type="button"
			class="btn btn-xs {sortMode === 'severity' ? 'btn-primary' : 'btn-ghost'}"
			onclick={() => (sortMode = 'severity')}
		>
			severity
		</button>
	</div>

	{#if !isConnected}
		<div class="flex items-center gap-3 rounded-2xl border border-warning/20 bg-warning/8 px-5 py-4 text-sm">
			<Unplug size={16} class="text-warning shrink-0" />
			<span class="opacity-70">Agent offline - incident feed unavailable. <a href={resolve('/settings#connection')} class="link link-primary">Configure connection</a> to load live data.</span>
		</div>
	{:else if filteredIncidents.length === 0}
		<p class="text-sm opacity-50">
			{query || severityFilter !== 'all' || statusFilter !== 'all'
				? labels.emptyFiltered
				: labels.emptyDefault}
		</p>
	{:else}
		<div class="space-y-3">
			{#each displayedIncidents as incident (incident.id)}
				<div class="bg-base-100/50 rounded-2xl border border-white/10 p-4">
					<div class="flex items-center justify-between gap-4">
						<div class="text-xs font-bold tracking-wider uppercase">{incident.type}</div>
						<div
							class="badge {incident.severity === 'critical'
								? 'badge-error'
								: incident.severity === 'high'
									? 'badge-warning'
									: 'badge-info'}"
						>
							{incident.severity}
						</div>
					</div>
					<div class="mt-2 text-sm opacity-80">{incident.message}</div>
					<div class="mt-2 text-[11px] opacity-40">{incident.nodeId} · {incident.status} · {formatTime(incident.timestamp)}</div>
				</div>
			{/each}
		</div>
	{/if}
</section>
