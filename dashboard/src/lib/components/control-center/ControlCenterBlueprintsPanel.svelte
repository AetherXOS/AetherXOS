<script lang="ts">
	import { m } from '$lib/paraglide/messages';
	import type { Blueprint } from '$lib/types';

	interface Props {
		blueprints: Blueprint[];
		isExecuting: boolean;
		categoryLabel: Record<Blueprint['category'], string>;
		onExecute: (id: string) => void;
	}

	let { blueprints, isExecuting, categoryLabel, onExecute }: Props = $props();
</script>

<div class="grid grid-cols-1 gap-5 md:grid-cols-2 xl:grid-cols-3">
	{#each blueprints as bp (bp.id)}
		<div class="card space-y-5 rounded-3xl border border-white/5 bg-base-200/80 p-6">
			<h3 class="text-lg font-black leading-tight">{bp.name}</h3>
			<div class="badge badge-outline">
				{m.control_blueprint_category()}: {categoryLabel[bp.category]}
			</div>
			<p class="text-sm leading-relaxed opacity-60">{bp.description}</p>
			<button class="btn btn-primary btn-sm" onclick={() => onExecute(bp.id)} disabled={isExecuting}>
				{m.control_execute()}
			</button>
		</div>
	{/each}
</div>
