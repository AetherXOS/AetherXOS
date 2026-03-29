<script lang="ts">
	import { resolve } from '$app/paths';
	import { m } from '$lib/paraglide/messages';
	import { Compass, FileSearch, Layers3, PlayCircle } from 'lucide-svelte';

	type DebugLayer = {
		id: string;
		title: string;
		detail: string;
		subsections: string[];
	};

	type DebugDefinition = {
		term: string;
		meaning: string;
	};

	const {
		debugLayers,
		debugDefinitions,
		onFocusCritical,
		onFocusErrors,
		onResetFilters
	}: {
		debugLayers: DebugLayer[];
		debugDefinitions: DebugDefinition[];
		onFocusCritical: () => void;
		onFocusErrors: () => void;
		onResetFilters: () => void;
	} = $props();
</script>

<section class="card rounded-3xl border border-white/5 bg-base-200/70 p-6 sm:p-8 space-y-5">
	<div class="flex items-center justify-between gap-4">
		<div>
			<div class="flex items-center gap-2 text-sm font-black tracking-widest uppercase opacity-60">
				<Layers3 size={15} />
				{m.debug_architecture_title()}
			</div>
			<div class="text-xs opacity-50">{m.debug_architecture_subtitle()}</div>
		</div>
		<div class="badge badge-outline">{debugLayers.length} {m.debug_layers_label()}</div>
	</div>

	<div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
		{#each debugLayers as layer (layer.id)}
			<div class="rounded-xl border border-white/10 bg-base-100/40 p-4 space-y-2 text-xs">
				<div class="font-black uppercase tracking-wide">{layer.title}</div>
				<div class="opacity-70">{layer.detail}</div>
				<div class="flex flex-wrap gap-2">
					{#each layer.subsections as subsection (subsection)}
						<span class="badge badge-ghost border border-white/10 px-3 py-2 text-[11px]">{subsection}</span>
					{/each}
				</div>
			</div>
		{/each}
	</div>

	<div class="grid grid-cols-1 gap-4 xl:grid-cols-[1.2fr_1fr]">
		<div class="rounded-xl border border-white/10 bg-base-100/40 p-4 space-y-3">
			<div class="flex items-center gap-2 text-sm font-black uppercase tracking-wide">
				<PlayCircle size={15} class="text-primary" />
				{m.debug_autonomy_title()}
			</div>
			<div class="grid grid-cols-1 gap-3 md:grid-cols-2 text-xs">
				<button class="rounded-xl border border-white/10 bg-base-100/45 p-3 text-left hover:border-primary/30" onclick={onFocusCritical}>
					<div class="font-black uppercase tracking-wide">{m.debug_autonomy_critical_title()}</div>
					<div class="mt-1 opacity-70">{m.debug_autonomy_critical_detail()}</div>
				</button>
				<button class="rounded-xl border border-white/10 bg-base-100/45 p-3 text-left hover:border-primary/30" onclick={onFocusErrors}>
					<div class="font-black uppercase tracking-wide">{m.debug_autonomy_error_title()}</div>
					<div class="mt-1 opacity-70">{m.debug_autonomy_error_detail()}</div>
				</button>
				<button class="rounded-xl border border-white/10 bg-base-100/45 p-3 text-left hover:border-primary/30" onclick={onResetFilters}>
					<div class="font-black uppercase tracking-wide">{m.debug_autonomy_reset_title()}</div>
					<div class="mt-1 opacity-70">{m.debug_autonomy_reset_detail()}</div>
				</button>
				<a href={resolve('/operations')} class="rounded-xl border border-white/10 bg-base-100/45 p-3 hover:border-primary/30">
					<div class="font-black uppercase tracking-wide">{m.debug_autonomy_ops_title()}</div>
					<div class="mt-1 opacity-70">{m.debug_autonomy_ops_detail()}</div>
				</a>
			</div>
		</div>

		<div class="rounded-xl border border-white/10 bg-base-100/40 p-4 space-y-3">
			<div class="flex items-center gap-2 text-sm font-black uppercase tracking-wide">
				<FileSearch size={15} class="text-primary" />
				{m.debug_definitions_title()}
			</div>
			<div class="space-y-2">
				{#each debugDefinitions as item (item.term)}
					<div class="rounded-xl border border-white/10 bg-base-100/45 p-3 text-xs">
						<div class="font-black uppercase tracking-wide">{item.term}</div>
						<div class="mt-1 opacity-70">{item.meaning}</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
</section>
