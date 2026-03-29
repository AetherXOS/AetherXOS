<script lang="ts">
	import type { AgentCatalogAction } from '$lib/types';

	interface Props {
		pendingAction: AgentCatalogAction | null;
		onCancel: () => void;
		onConfirm: () => void;
	}

	let { pendingAction, onCancel, onConfirm }: Props = $props();
</script>

{#if pendingAction}
	<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/55 p-4">
		<div class="w-full max-w-lg rounded-3xl border border-white/10 bg-base-200 p-6 shadow-2xl">
			<div class="text-xs font-black uppercase tracking-[0.3em] text-warning">High Risk Action</div>
			<h3 class="mt-3 text-2xl font-black">{pendingAction.title}</h3>
			<p class="mt-2 text-sm opacity-70">{pendingAction.desc}</p>
			{#if pendingAction.impact}
				<p class="mt-3 text-sm text-warning/90">{pendingAction.impact}</p>
			{/if}
			<div class="mt-6 flex justify-end gap-2">
				<button class="btn btn-ghost" onclick={onCancel}>Cancel</button>
				<button class="btn btn-warning" onclick={onConfirm}>Confirm and run</button>
			</div>
		</div>
	</div>
{/if}
