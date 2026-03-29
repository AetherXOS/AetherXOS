<script lang="ts">
	import type { ConfigDriftReport } from '$lib/types';
	import type { DriftApplyMode } from './types';
	import { RefreshCw } from 'lucide-svelte';

	interface Props {
		driftApplyMode: DriftApplyMode;
		driftReport: ConfigDriftReport | null;
		busy: boolean;
		onDriftApplyModeChange: (value: DriftApplyMode) => void;
		onRefreshInsights: () => void;
		onApplyDriftFix: () => void;
	}

	let { driftApplyMode, driftReport, busy, onDriftApplyModeChange, onRefreshInsights, onApplyDriftFix }: Props = $props();
</script>

<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
	<div class="flex items-center justify-between gap-3">
		<div>
			<div class="text-sm font-black uppercase tracking-wider opacity-70">Drift repair</div>
			<div class="text-xs opacity-55">compare current cargo features against recommended build goals</div>
		</div>
		<button class="btn btn-outline btn-sm" onclick={onRefreshInsights} disabled={busy}>
			<RefreshCw size={14} /> Refresh all
		</button>
	</div>
	<div class="mt-4 flex flex-wrap items-center gap-2">
		<select class="select select-sm bg-base-100" value={driftApplyMode} onchange={(event) => onDriftApplyModeChange((event.currentTarget as HTMLSelectElement).value as DriftApplyMode)}>
			<option value="missing_only">missing_only</option>
			<option value="full">full</option>
		</select>
		<button class="btn btn-primary btn-sm" onclick={onApplyDriftFix} disabled={busy}>Repair drift</button>
	</div>
	<div class="mt-4 space-y-3">
		{#each driftReport?.goals ?? [] as goal (goal.goal)}
			<div class="rounded-2xl border border-white/8 bg-base-100/30 p-3">
				<div class="flex items-center justify-between gap-2">
					<div class="font-semibold">{goal.goal}</div>
					<div class="flex gap-2 text-xs">
						<div class="badge badge-warning">missing {goal.missingCount}</div>
						<div class="badge badge-ghost">extra {goal.extraCount}</div>
					</div>
				</div>
				{#if goal.missingCount > 0}
					<div class="mt-2 text-[11px] opacity-60">{goal.missing.join(', ')}</div>
				{/if}
			</div>
		{/each}
	</div>
</div>
