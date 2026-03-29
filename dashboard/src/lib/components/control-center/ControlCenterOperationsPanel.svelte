<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { AgentCatalogAction, AgentJobSummary } from '$lib/types';
	import type { ControlPriority } from './types';
	import { Wrench, Hammer, RefreshCw } from 'lucide-svelte';

	interface Props {
		rows: AgentCatalogAction[];
		categories: string[];
		operationSearch: string;
		operationCategory: string;
		operationPriority: ControlPriority;
		operationMessage: string;
		operationsBusy: boolean;
		jobsBusy: boolean;
		jobs: AgentJobSummary[];
		onSearchChange: (value: string) => void;
		onCategoryChange: (value: string) => void;
		onPriorityChange: (value: ControlPriority) => void;
		onRefreshCatalog: () => void;
		onRequestOperation: (action: AgentCatalogAction) => void;
		onRefreshJobs: () => void;
		onCancelJob: (id: string) => void;
	}

	let {
		rows,
		categories,
		operationSearch,
		operationCategory,
		operationPriority,
		operationMessage,
		operationsBusy,
		jobsBusy,
		jobs,
		onSearchChange,
		onCategoryChange,
		onPriorityChange,
		onRefreshCatalog,
		onRequestOperation,
		onRefreshJobs,
		onCancelJob
	}: Props = $props();

	const runningJobs = $derived.by(() => jobs.filter((job) => job.status === 'running').length);
	const queuedJobs = $derived.by(() => jobs.filter((job) => job.status === 'queued').length);
	const completedJobs = $derived.by(() =>
		jobs.filter((job) => job.status === 'completed' || job.status === 'failed' || typeof job.exit_code === 'number').length
	);
</script>

<section class="space-y-4">
	<div class="grid grid-cols-1 gap-3 lg:grid-cols-3">
		<div class="rounded-2xl border border-white/8 bg-base-200/70 p-4">
			<div class="text-[11px] font-black uppercase tracking-[0.25em] opacity-45">Catalog</div>
			<div class="mt-3 text-3xl font-black tracking-tight">{rows.length}</div>
			<div class="text-sm opacity-60">action endpoints exposed to the dashboard</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-200/70 p-4">
			<div class="text-[11px] font-black uppercase tracking-[0.25em] opacity-45">Active jobs</div>
			<div class="mt-3 flex items-baseline gap-3">
				<div class="text-3xl font-black tracking-tight">{runningJobs}</div>
				<div class="text-sm opacity-55">running</div>
			</div>
			<div class="mt-2 text-sm opacity-60">{queuedJobs} queued</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-200/70 p-4">
			<div class="text-[11px] font-black uppercase tracking-[0.25em] opacity-45">History</div>
			<div class="mt-3 text-3xl font-black tracking-tight">{completedJobs}</div>
			<div class="text-sm opacity-60">completed or exited jobs in memory</div>
		</div>
	</div>

	<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
		<div class="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
			<div class="flex items-center gap-3 text-sm font-black tracking-wider uppercase opacity-70">
				<Wrench size={16} />
				{m.control_operations_title()}
			</div>
			<div class="flex flex-wrap items-center gap-2">
				<input
					class="input input-sm bg-base-100 min-w-48"
					placeholder="Search actions"
					value={operationSearch}
					oninput={(event) => onSearchChange((event.currentTarget as HTMLInputElement).value)}
				/>
				<select
					class="select select-sm bg-base-100"
					value={operationCategory}
					onchange={(event) => onCategoryChange((event.currentTarget as HTMLSelectElement).value)}
				>
					<option value="all">all categories</option>
					{#each categories as category (category)}
						<option value={category}>{category}</option>
					{/each}
				</select>
				<span class="text-[11px] font-black tracking-wider uppercase opacity-50">{m.control_priority_label()}</span>
				<select
					class="select select-sm bg-base-100 min-w-30"
					value={operationPriority}
					onchange={(event) =>
						onPriorityChange((event.currentTarget as HTMLSelectElement).value as ControlPriority)}
				>
					<option value="high">{m.control_priority_high()}</option>
					<option value="normal">{m.control_priority_normal()}</option>
					<option value="low">{m.control_priority_low()}</option>
				</select>
				<button class="btn btn-outline btn-sm" onclick={onRefreshCatalog} disabled={operationsBusy}>
					{m.control_refresh_catalog()}
				</button>
			</div>
		</div>
		{#if operationMessage}
			<div class="alert alert-info mt-4 py-2 text-sm">
				<span>{operationMessage}</span>
			</div>
		{/if}
	</div>

	{#if rows.length === 0}
		<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 text-sm opacity-70">
			No catalog actions matched the current filter.
		</div>
	{:else}
		<div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
			{#each rows as row (row.id)}
				<div class="card rounded-2xl border border-white/8 bg-base-200/70 p-5">
					<div class="flex items-start justify-between gap-3">
						<div>
							<h3 class="text-base font-black tracking-tight">{row.title}</h3>
							<p class="mt-1 text-sm opacity-65">{row.desc}</p>
						</div>
						<div class="badge badge-outline">{row.risk}</div>
					</div>
					<div class="mt-3 text-[11px] font-medium opacity-50">{row.category}</div>
					{#if row.impact}
						<div class="mt-2 text-[12px] opacity-55">{row.impact}</div>
					{/if}
					<div class="mt-4">
						<button class="btn btn-primary btn-sm w-full" onclick={() => onRequestOperation(row)} disabled={operationsBusy}>
							<Hammer size={14} />
							{m.control_execute()}
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
		<div class="mb-4 flex items-center justify-between gap-3">
			<div class="text-sm font-black tracking-wider uppercase opacity-70">Job Monitor</div>
			<button class="btn btn-outline btn-sm" onclick={onRefreshJobs} disabled={jobsBusy}>
				<RefreshCw size={14} />
				Refresh jobs
			</button>
		</div>
		<div class="space-y-3">
			{#if jobs.length === 0}
				<div class="text-sm opacity-60">No jobs yet.</div>
			{:else}
				{#each jobs.slice(0, 12) as job (job.id)}
					<div class="flex flex-col gap-3 rounded-2xl border border-white/10 bg-base-100/30 p-4 xl:flex-row xl:items-center xl:justify-between">
						<div class="space-y-1">
							<div class="text-sm font-semibold">{job.action}</div>
							<div class="text-[11px] opacity-55">{job.id}</div>
							<div class="text-[11px] opacity-45">
								queued {job.queued_utc ?? '-'}
								{#if job.started_utc}
									| started {job.started_utc}
								{/if}
								{#if job.finished_utc}
									| finished {job.finished_utc}
								{/if}
							</div>
						</div>
						<div class="flex flex-wrap items-center gap-2 text-xs">
							<div class="badge badge-outline">{job.status}</div>
							<div class="badge badge-ghost">{job.priority}</div>
							{#if typeof job.exit_code === 'number'}
								<div class="badge badge-ghost">exit {job.exit_code}</div>
							{/if}
						</div>
						{#if job.status === 'queued' || job.status === 'running'}
							<button class="btn btn-outline btn-sm" onclick={() => onCancelJob(job.id)} disabled={jobsBusy}>
								Cancel
							</button>
						{/if}
					</div>
				{/each}
			{/if}
		</div>
	</div>
</section>
