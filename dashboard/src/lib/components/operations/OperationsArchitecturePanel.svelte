<script lang="ts">
	import { FileText, Layers3, PlayCircle } from 'lucide-svelte';

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
		offline: boolean;
		isRefreshing: boolean;
		labels: {
			title: string;
			subtitle: string;
			layersLabel: string;
			autonomyTitle: string;
			autoCriticalTitle: string;
			autoCriticalDetail: string;
			autoRefreshTitle: string;
			autoRefreshDetail: string;
			definitionsTitle: string;
		};
		onFocusCritical: () => void;
		onRefresh: () => void;
	}

	let { layers, definitions, offline, isRefreshing, labels, onFocusCritical, onRefresh }: Props =
		$props();
</script>

<section class="card bg-base-200 space-y-5 border border-white/5 p-6">
	<div class="flex items-center justify-between gap-4">
		<div>
			<div class="flex items-center gap-2 text-sm font-black tracking-widest uppercase opacity-60">
				<Layers3 size={15} />
				{labels.title}
			</div>
			<div class="text-xs opacity-50">{labels.subtitle}</div>
		</div>
		<div class="badge badge-outline">{layers.length} {labels.layersLabel}</div>
	</div>

	<div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
		{#each layers as layer (layer.id)}
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
				{labels.autonomyTitle}
			</div>
			<div class="grid grid-cols-1 gap-3 md:grid-cols-2 text-xs">
				<button class="rounded-xl border border-white/10 bg-base-100/40 p-3 text-left hover:border-primary/30" onclick={onFocusCritical} disabled={offline}>
					<div class="font-black uppercase tracking-wide">{labels.autoCriticalTitle}</div>
					<div class="mt-1 opacity-70">{labels.autoCriticalDetail}</div>
				</button>
				<button class="rounded-xl border border-white/10 bg-base-100/40 p-3 text-left hover:border-primary/30" onclick={onRefresh} disabled={isRefreshing}>
					<div class="font-black uppercase tracking-wide">{labels.autoRefreshTitle}</div>
					<div class="mt-1 opacity-70">{labels.autoRefreshDetail}</div>
				</button>
			</div>
		</div>

		<div class="rounded-xl border border-white/10 bg-base-100/40 p-4 space-y-3">
			<div class="flex items-center gap-2 text-sm font-black uppercase tracking-wide">
				<FileText size={15} class="text-primary" />
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
