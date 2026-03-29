<script lang="ts">
	import { resolve } from '$app/paths';
	import { PlayCircle, ShieldAlert } from 'lucide-svelte';

	interface LayerItem {
		id: string;
		title: string;
		detail: string;
		subsections: string[];
	}

	interface DefinitionItem {
		term: string;
		meaning: string;
	}

	interface Props {
		layers: LayerItem[];
		definitions: DefinitionItem[];
		isConnected: boolean;
		hasCriticalIncidents: boolean;
		labels: {
			title: string;
			subtitle: string;
			layersLabel: string;
			autonomyTitle: string;
			autonomyIncidentsTitle: string;
			autonomyIncidentsDetail: string;
			autonomyRecoverTitle: string;
			autonomyRecoverDetail: string;
			autonomyCriticalTitle: string;
			autonomyCriticalDetail: string;
			autonomyProactiveTitle: string;
			autonomyProactiveDetail: string;
			definitionsTitle: string;
		};
	}

	let { layers, definitions, isConnected, hasCriticalIncidents, labels }: Props = $props();
</script>

<section class="rounded-3xl border border-white/5 bg-base-200/70 p-6 sm:p-8 space-y-5">
	<div class="flex items-center justify-between gap-4">
		<div>
			<div class="text-sm font-black tracking-widest uppercase opacity-60">{labels.title}</div>
			<div class="text-xs opacity-50">{labels.subtitle}</div>
		</div>
		<div class="badge badge-outline">{layers.length} {labels.layersLabel}</div>
	</div>

	<div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
		{#each layers as layer (layer.id)}
			<div class="rounded-xl border border-white/10 bg-base-100/40 p-4 space-y-3 text-xs">
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
				{labels.autonomyTitle}
			</div>
			<div class="grid grid-cols-1 gap-3 md:grid-cols-2 text-xs">
				<a href={resolve(isConnected ? '/operations' : '/settings#verify')} class="rounded-xl border border-white/10 bg-base-100/40 p-3 hover:border-primary/30">
					<div class="font-black uppercase tracking-wide">{isConnected ? labels.autonomyIncidentsTitle : labels.autonomyRecoverTitle}</div>
					<div class="mt-1 opacity-70">{isConnected ? labels.autonomyIncidentsDetail : labels.autonomyRecoverDetail}</div>
				</a>
				<a href={resolve(hasCriticalIncidents ? '/operations' : '/control-center')} class="rounded-xl border border-white/10 bg-base-100/40 p-3 hover:border-primary/30">
					<div class="font-black uppercase tracking-wide">{hasCriticalIncidents ? labels.autonomyCriticalTitle : labels.autonomyProactiveTitle}</div>
					<div class="mt-1 opacity-70">{hasCriticalIncidents ? labels.autonomyCriticalDetail : labels.autonomyProactiveDetail}</div>
				</a>
			</div>
		</div>

		<div class="rounded-xl border border-white/10 bg-base-100/40 p-4 space-y-3">
			<div class="flex items-center gap-2 text-sm font-black uppercase tracking-wide">
				<ShieldAlert size={15} class="text-primary" />
				{labels.definitionsTitle}
			</div>
			<div class="space-y-2">
				{#each definitions as item (item.term)}
					<div class="rounded-xl border border-white/10 bg-base-100/45 p-3 text-xs">
						<div class="font-black uppercase tracking-wide">{item.term}</div>
						<div class="mt-1 opacity-70">{item.meaning}</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
</section>
