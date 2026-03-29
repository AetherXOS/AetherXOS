<script lang="ts">
	import type { AgentHost, AgentJobDetail, AgentJobSummary, JobStreamTimelineEvent } from '$lib/types';
	import JobOutputViewer from './JobOutputViewer.svelte';

	interface Props {
		hosts: AgentHost[];
		selectedJobHostId: string;
		jobs: AgentJobSummary[];
		selectedJobId: string;
		selectedJobTail: number;
		selectedJobDetail: AgentJobDetail | null;
		jobStreamEnabled: boolean;
		jobStreamConnected: boolean;
		jobStreamStatus: 'idle' | 'connecting' | 'streaming' | 'fallback' | 'error';
		jobAutoRefreshEnabled: boolean;
		jobAutoRefreshMs: number;
		streamEvents: JobStreamTimelineEvent[];
		streamReconnectCount: number;
		busy: boolean;
		onSelectedJobHostChange: (value: string) => void;
		onRefreshHosts: () => void;
		onSelectedJobChange: (value: string) => void;
		onSelectedJobTailChange: (value: number) => void;
		onFetchJobDetail: (id: string, tail: number) => void;
		onJobStreamEnabledChange: (checked: boolean) => void;
		onJobAutoRefreshEnabledChange: (checked: boolean) => void;
		onJobAutoRefreshMsChange: (value: number) => void;
		onCancelJob: () => void;
	}

	let {
		hosts,
		selectedJobHostId,
		jobs,
		selectedJobId,
		selectedJobTail,
		selectedJobDetail,
		jobStreamEnabled,
		jobStreamConnected,
		jobStreamStatus,
		jobAutoRefreshEnabled,
		jobAutoRefreshMs,
		streamEvents,
		streamReconnectCount,
		busy,
		onSelectedJobHostChange,
		onRefreshHosts,
		onSelectedJobChange,
		onSelectedJobTailChange,
		onFetchJobDetail,
		onJobStreamEnabledChange,
		onJobAutoRefreshEnabledChange,
		onJobAutoRefreshMsChange,
		onCancelJob
	}: Props = $props();

	let jobStatusFilter = $state<'all' | 'running' | 'queued' | 'failed' | 'completed'>('all');
	let showTimeline = $state(false);

	type OutputLine = { raw: string; severity: 'error' | 'warn' | 'info' | 'normal' };

	const selectedJobRow = $derived.by(() => jobs.find((job) => job.id === selectedJobId) ?? null);
	const selectedHost = $derived.by(
		() => hosts.find((host) => host.id === selectedJobHostId) ?? null
	);
	const hostOptions = $derived.by(() =>
		hosts.length > 0 ? hosts : [{ id: 'local', name: 'Local', url: '', enabled: true }]
	);

	const filteredJobs = $derived.by(() => {
		if (jobStatusFilter === 'all') return jobs;
		return jobs.filter((j) => {
			const s = j.status.toLowerCase();
			if (jobStatusFilter === 'running') return s === 'running';
			if (jobStatusFilter === 'queued') return s === 'queued' || s === 'pending';
			if (jobStatusFilter === 'failed') return s === 'failed' || j.ok === false || j.exit_code !== undefined && j.exit_code !== null && j.exit_code !== 0;
			if (jobStatusFilter === 'completed') return s === 'done' || s === 'completed' || s === 'success' || j.ok === true;
			return true;
		});
	});

	const outputLines = $derived.by((): OutputLine[] => {
		const raw = selectedJobDetail?.output ?? '';
		if (!raw) return [];
		return raw.split('\n').map((line) => {
			const lower = line.toLowerCase();
			if (/\b(error|fail|fatal|panic|abort|critical|exception)\b/.test(lower))
				return { raw: line, severity: 'error' as const };
			if (/\b(warn|warning|deprecated|caution)\b/.test(lower))
				return { raw: line, severity: 'warn' as const };
			if (/\b(info|ok|success|done|complete|pass|built|compil)\b/.test(lower))
				return { raw: line, severity: 'info' as const };
			return { raw: line, severity: 'normal' as const };
		});
	});

	const streamBadgeClass = $derived.by(() => {
		if (jobStreamStatus === 'streaming') return 'badge-success';
		if (jobStreamStatus === 'connecting') return 'badge-warning';
		if (jobStreamStatus === 'fallback') return 'badge-info';
		if (jobStreamStatus === 'error') return 'badge-error';
		return 'badge-ghost';
	});
	const streamLabel = $derived.by(() => {
		if (jobStreamStatus === 'streaming') return 'live stream';
		if (jobStreamStatus === 'connecting') return 'connecting';
		if (jobStreamStatus === 'fallback') return 'fallback polling';
		if (jobStreamStatus === 'error') return 'stream error';
		return 'idle';
	});

	const jobDuration = $derived.by(() => {
		const job = selectedJobDetail?.job ?? selectedJobRow;
		if (!job) return '-';
		if (job.duration_ms !== undefined && job.duration_ms !== null) {
			const ms = job.duration_ms;
			if (ms < 1000) return `${ms}ms`;
			if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
			return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
		}
		if (job.started_utc) {
			const start = new Date(job.started_utc).getTime();
			const end = job.finished_utc ? new Date(job.finished_utc).getTime() : Date.now();
			const ms = end - start;
			if (ms < 0) return '-';
			if (ms < 1000) return `${ms}ms`;
			if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
			return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
		}
		return '-';
	});

	const timelineEventClass = (type: JobStreamTimelineEvent['type']): string => {
		if (type === 'error') return 'badge-error';
		if (type === 'done') return 'badge-success';
		if (type === 'open') return 'badge-success';
		if (type === 'close' || type === 'abort') return 'badge-ghost';
		if (type === 'reconnect') return 'badge-warning';
		if (type === 'timeout') return 'badge-warning';
		if (type === 'tail') return 'badge-info';
		return 'badge-ghost';
	};

	const isJobRunning = $derived.by(() => {
		const s = (selectedJobDetail?.job.status ?? selectedJobRow?.status ?? '').toLowerCase();
		return s === 'running' || s === 'queued' || s === 'pending';
	});

	const TAIL_PRESETS = [50, 200, 400, 1000] as const;
</script>

<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
	<div class="flex items-center justify-between">
		<div class="text-sm font-black uppercase tracking-wider opacity-70">Job output inspector</div>
		<div class="flex items-center gap-2">
			<div class={`badge badge-sm ${streamBadgeClass}`}>{streamLabel}</div>
			{#if streamReconnectCount > 0}
				<div class="badge badge-warning badge-sm">{streamReconnectCount}x reconnect</div>
			{/if}
		</div>
	</div>

	<!-- Host selector row -->
	<div class="mt-4 flex flex-wrap items-center gap-2">
		<select class="select select-sm bg-base-100 min-w-44" value={selectedJobHostId} onchange={(event) => onSelectedJobHostChange((event.currentTarget as HTMLSelectElement).value)}>
			{#each hostOptions as host (host.id)}
				<option value={host.id}>{host.name ?? host.id} ({host.id})</option>
			{/each}
		</select>
		<button class="btn btn-ghost btn-sm" onclick={onRefreshHosts} disabled={busy}>↺ Hosts</button>
		<!-- Job status filter -->
		<select class="select select-sm bg-base-100" value={jobStatusFilter} onchange={(event) => (jobStatusFilter = (event.currentTarget as HTMLSelectElement).value as typeof jobStatusFilter)}>
			<option value="all">All jobs</option>
			<option value="running">Running</option>
			<option value="queued">Queued</option>
			<option value="completed">Completed</option>
			<option value="failed">Failed</option>
		</select>
		<select class="select select-sm bg-base-100 min-w-72" value={selectedJobId} onchange={(event) => onSelectedJobChange((event.currentTarget as HTMLSelectElement).value)}>
			<option value="">select job</option>
			{#each filteredJobs as job (job.id)}
				<option value={job.id}>{job.action} [{job.status}] {job.id}</option>
			{/each}
		</select>
	</div>

	<!-- Host health mini panel -->
	{#if selectedHost}
		<div class="mt-3 flex flex-wrap items-center gap-2 rounded-2xl border border-white/8 bg-base-100/20 px-3 py-2 text-xs">
			<span class="opacity-45 mr-1">host</span>
			{#if selectedHost.reachable === true}
				<div class="badge badge-success badge-xs">reachable</div>
			{:else if selectedHost.reachable === false}
				<div class="badge badge-error badge-xs">unreachable</div>
			{:else}
				<div class="badge badge-ghost badge-xs">status unknown</div>
			{/if}
			{#if selectedHost.role}
				<div class="badge badge-outline badge-xs">{selectedHost.role}</div>
			{:else if selectedHost.roleHint}
				<div class="badge badge-outline badge-xs">{selectedHost.roleHint}</div>
			{/if}
			{#if selectedHost.busy}
				<div class="badge badge-warning badge-xs">busy</div>
			{/if}
			{#if selectedHost.runningCount !== undefined && selectedHost.runningCount !== null}
				<span class="opacity-70"><span class="font-bold">{selectedHost.runningCount}</span> running</span>
			{/if}
			{#if selectedHost.queueCount !== undefined && selectedHost.queueCount !== null}
				<span class="opacity-70"><span class="font-bold">{selectedHost.queueCount}</span> queued</span>
			{/if}
			{#if selectedHost.url}
				<span class="opacity-40 font-mono">{selectedHost.url}</span>
			{/if}
			{#if selectedHost.error}
				<span class="text-error">{selectedHost.error}</span>
			{/if}
		</div>
	{/if}

	<!-- Tail + snapshot row -->
	<div class="mt-3 flex flex-wrap items-center gap-2">
		<span class="text-xs opacity-50">tail</span>
		{#each TAIL_PRESETS as p (p)}
			<button
				class={`btn btn-xs ${selectedJobTail === p ? 'btn-primary' : 'btn-ghost'}`}
				onclick={() => onSelectedJobTailChange(p)}
				disabled={busy}>{p}</button>
		{/each}
		<input class="input input-sm bg-base-100 w-24" type="number" min="20" max="4000" step="20" value={selectedJobTail} oninput={(event) => onSelectedJobTailChange(Number.parseInt((event.currentTarget as HTMLInputElement).value || '400', 10))} />
		<button class="btn btn-outline btn-sm" onclick={() => selectedJobId && onFetchJobDetail(selectedJobId, selectedJobTail)} disabled={busy || !selectedJobId}>Load snapshot</button>
		{#if isJobRunning && selectedJobId}
			<button class="btn btn-error btn-sm" onclick={onCancelJob} disabled={busy}>✕ Cancel job</button>
		{/if}
	</div>

	<!-- Stream controls row -->
	<div class="mt-3 flex flex-wrap items-center gap-4 rounded-2xl border border-white/8 bg-base-100/30 p-3">
		<label class="label cursor-pointer gap-2 py-0">
			<span class="label-text text-xs opacity-80">live stream</span>
			<input class="toggle toggle-xs" type="checkbox" checked={jobStreamEnabled} onchange={(event) => onJobStreamEnabledChange((event.currentTarget as HTMLInputElement).checked)} />
		</label>
		<div class="text-xs opacity-70">status: {streamLabel}{jobStreamConnected ? ' (connected)' : ''}</div>
		{#if selectedHost}
			<div class="badge badge-outline badge-sm">host {selectedHost.id}</div>
		{/if}
		<label class="label cursor-pointer gap-2 py-0">
			<span class="label-text text-xs opacity-70">polling fallback</span>
			<input class="checkbox checkbox-xs" type="checkbox" checked={jobAutoRefreshEnabled} onchange={(event) => onJobAutoRefreshEnabledChange((event.currentTarget as HTMLInputElement).checked)} />
		</label>
		<div class="flex items-center gap-2 text-xs opacity-70">
			<span>interval ms</span>
			<input class="input input-xs bg-base-100 w-24" type="number" min="1500" max="30000" step="500" value={jobAutoRefreshMs} oninput={(event) => onJobAutoRefreshMsChange(Number.parseInt((event.currentTarget as HTMLInputElement).value || '5000', 10))} />
		</div>
		<button class="btn btn-ghost btn-xs ml-auto" onclick={() => (showTimeline = !showTimeline)}>{showTimeline ? 'Hide' : 'Show'} timeline</button>
	</div>

	<!-- Stream event timeline -->
	{#if showTimeline}
		<div class="mt-2 rounded-2xl border border-white/8 bg-base-100/20 p-3">
			<div class="text-[11px] font-bold uppercase tracking-widest opacity-40 mb-2">Stream timeline</div>
			{#if streamEvents.length === 0}
				<div class="text-xs opacity-40 italic">No events yet.</div>
			{:else}
				<div class="flex flex-col gap-1">
					{#each streamEvents.slice().reverse() as ev (ev.time + ev.type + ev.label)}
						<div class="flex items-center gap-2 text-xs">
							<span class="font-mono opacity-40 shrink-0">{ev.time}</span>
							<div class={`badge badge-xs shrink-0 ${timelineEventClass(ev.type)}`}>{ev.type}</div>
							<span class="opacity-70 truncate">{ev.label}</span>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	<!-- Extended metadata grid -->
	<div class="mt-4 grid grid-cols-2 gap-2 text-sm sm:grid-cols-3 xl:grid-cols-6">
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-2">
			<div class="text-[10px] opacity-50">Status</div>
			<div class="mt-0.5 font-black text-xs">{selectedJobDetail?.job.status ?? selectedJobRow?.status ?? '-'}</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-2">
			<div class="text-[10px] opacity-50">Priority</div>
			<div class="mt-0.5 font-black text-xs">{selectedJobDetail?.job.priority ?? selectedJobRow?.priority ?? '-'}</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-2">
			<div class="text-[10px] opacity-50">Exit code</div>
			<div class="mt-0.5 font-black text-xs">{selectedJobDetail?.job.exit_code ?? selectedJobRow?.exit_code ?? '-'}</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-2">
			<div class="text-[10px] opacity-50">Duration</div>
			<div class="mt-0.5 font-black text-xs">{jobDuration}</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-2 col-span-2 sm:col-span-1">
			<div class="text-[10px] opacity-50">Action</div>
			<div class="mt-0.5 font-mono text-xs truncate" title={selectedJobDetail?.job.action ?? selectedJobRow?.action}>{selectedJobDetail?.job.action ?? selectedJobRow?.action ?? '-'}</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-2">
			<div class="text-[10px] opacity-50">Source</div>
			<div class="mt-0.5 font-mono text-xs truncate">{selectedJobDetail?.job.source ?? selectedJobRow?.source ?? '-'}</div>
		</div>
	</div>

	<!-- Timestamps row -->
	{#if selectedJobDetail?.job.started_utc || selectedJobDetail?.job.queued_utc}
		<div class="mt-2 flex flex-wrap gap-3 text-xs opacity-50">
			{#if selectedJobDetail.job.queued_utc}
				<span>queued {new Date(selectedJobDetail.job.queued_utc).toLocaleTimeString()}</span>
			{/if}
			{#if selectedJobDetail.job.started_utc}
				<span>started {new Date(selectedJobDetail.job.started_utc).toLocaleTimeString()}</span>
			{/if}
			{#if selectedJobDetail.job.finished_utc}
				<span>finished {new Date(selectedJobDetail.job.finished_utc).toLocaleTimeString()}</span>
			{/if}
		</div>
	{/if}

	<!-- Output viewer -->
	<JobOutputViewer {outputLines} />
</div>
