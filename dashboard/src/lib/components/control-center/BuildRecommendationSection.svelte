<script lang="ts">
	import type { BuildFeatureRecommendation } from '$lib/types';
	import type { ComposeGoal } from './types';
	import { RefreshCw } from 'lucide-svelte';

	interface Props {
		composeGoal: ComposeGoal;
		composeMinimal: boolean;
		recommendation: BuildFeatureRecommendation | null;
		busy: boolean;
		onComposeGoalChange: (value: ComposeGoal) => void;
		onComposeMinimalChange: (checked: boolean) => void;
		onRefreshRecommendation: () => void;
		onApplyCompose: () => void;
	}

	let {
		composeGoal,
		composeMinimal,
		recommendation,
		busy,
		onComposeGoalChange,
		onComposeMinimalChange,
		onRefreshRecommendation,
		onApplyCompose
	}: Props = $props();
</script>

<div class="card rounded-3xl border border-white/5 bg-base-200/80 p-5 sm:p-6">
	<div class="flex items-center justify-between gap-3">
		<div>
			<div class="text-sm font-black uppercase tracking-wider opacity-70">Build recommendation</div>
			<div class="text-xs opacity-55">preview feature selection before applying it</div>
		</div>
		<button class="btn btn-outline btn-sm" onclick={onRefreshRecommendation} disabled={busy}>
			<RefreshCw size={14} /> Refresh
		</button>
	</div>
	<div class="mt-4 flex flex-wrap items-center gap-2">
		<select class="select select-sm bg-base-100" value={composeGoal} onchange={(event) => onComposeGoalChange((event.currentTarget as HTMLSelectElement).value as ComposeGoal)}>
			<option value="boot_min">boot_min</option>
			<option value="linux_full">linux_full</option>
			<option value="release_hardening">release_hardening</option>
		</select>
		<label class="label cursor-pointer gap-2 py-0">
			<span class="label-text text-xs opacity-70">minimal</span>
			<input class="checkbox checkbox-xs" type="checkbox" checked={composeMinimal} onchange={(event) => onComposeMinimalChange((event.currentTarget as HTMLInputElement).checked)} />
		</label>
		<button class="btn btn-primary btn-sm" onclick={onApplyCompose} disabled={busy}>Apply recommendation</button>
	</div>
	<div class="mt-4 grid grid-cols-2 gap-3 text-sm">
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-3">
			<div class="opacity-55">Selected</div>
			<div class="mt-1 text-2xl font-black">{recommendation?.selectedCount ?? 0}</div>
		</div>
		<div class="rounded-2xl border border-white/8 bg-base-100/30 p-3">
			<div class="opacity-55">Available</div>
			<div class="mt-1 text-2xl font-black">{recommendation?.availableCount ?? 0}</div>
		</div>
	</div>
	{#if recommendation}
		<div class="mt-4 text-xs opacity-65">no_default_features: {String(recommendation.noDefaultFeatures)}</div>
		<div class="mt-3 max-h-40 overflow-auto rounded-2xl border border-white/8 bg-base-100/30 p-3 text-xs leading-6">
			{recommendation.selectedFeatures.join(', ')}
		</div>
	{/if}
</div>
