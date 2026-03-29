<script lang="ts">
	import { CheckCircle, ChevronRight } from 'lucide-svelte';

	interface SetupStep {
		id: string;
		label: string;
		hint: string;
		done: boolean;
	}

	interface ConfigSection {
		id: string;
		label: string;
	}

	interface ConfigCategory {
		id: string;
		title: string;
		detail: string;
		subsections: ConfigSection[];
	}

	interface Props {
		setupSteps: SetupStep[];
		setupProgress: number;
		configMap: ConfigCategory[];
	}

	let { setupSteps, setupProgress, configMap }: Props = $props();
</script>

<section class="card rounded-2xl border border-white/5 bg-base-200/70 p-5 sm:p-6 space-y-5">
	<div class="flex items-center justify-between gap-4">
		<div>
			<div class="text-sm font-black uppercase tracking-wider">Setup Navigator</div>
			<div class="text-xs opacity-50">Follow these layers in order. Each layer unlocks easier usage.</div>
		</div>
		<div class="text-xs font-mono opacity-60">{setupProgress}% complete</div>
	</div>
	<div class="h-2 overflow-hidden rounded-full bg-base-100/70">
		<div class="bg-primary h-full rounded-full transition-all duration-700" style="width: {setupProgress}%"></div>
	</div>
	<div class="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-4">
		{#each setupSteps as step (step.id)}
			<a href="#{step.id}" class="rounded-xl border px-3 py-3 text-xs transition-all {step.done ? 'border-success/25 bg-success/10' : 'border-white/10 bg-base-100/40 hover:border-primary/30'}">
				<div class="flex items-center gap-2 font-black uppercase tracking-wide">
					{#if step.done}
						<CheckCircle size={13} class="text-success" />
					{:else}
						<ChevronRight size={13} class="opacity-40" />
					{/if}
					{step.label}
				</div>
				<div class="mt-1 opacity-60">{step.hint}</div>
			</a>
		{/each}
	</div>
</section>

<section class="card rounded-2xl border border-white/5 bg-base-200/70 p-5 sm:p-6 space-y-5">
	<div class="flex items-center justify-between gap-4">
		<div>
			<div class="text-sm font-black uppercase tracking-wider">Configuration Architecture</div>
			<div class="text-xs opacity-50">Categorized layers with sub-sections for fast orientation.</div>
		</div>
		<div class="badge badge-outline">{configMap.length} categories</div>
	</div>
	<div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
		{#each configMap as category (category.id)}
			<div class="rounded-xl border border-white/10 bg-base-100/45 p-4 space-y-3">
				<div>
					<a href="#{category.id}" class="text-sm font-black uppercase tracking-wide hover:text-primary">{category.title}</a>
					<div class="mt-1 text-xs opacity-60">{category.detail}</div>
				</div>
				<div class="flex flex-wrap gap-2">
					{#each category.subsections as section (section.id)}
						<a href="#{section.id}" class="badge badge-ghost border border-white/10 px-3 py-2 text-[11px] hover:border-primary/30 hover:text-primary">
							{section.label}
						</a>
					{/each}
				</div>
			</div>
		{/each}
	</div>
</section>
