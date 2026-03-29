<script lang="ts">
	import { BookOpen, Bot, Layers3, PlayCircle, RefreshCw, Rocket, Wrench } from 'lucide-svelte';

	type FlowSection = {
		id: string;
		tab: string;
		title: string;
		detail: string;
		subsections: string[];
	};

	type GlossaryItem = {
		term: string;
		meaning: string;
	};

	type AutonomyAction = {
		id: string;
		tone: 'success' | 'warning' | 'error' | 'info';
		title: string;
		detail: string;
		cta: string;
	};

	type WorkflowStage = 'idle' | 'running' | 'done' | 'failed';

	type WorkflowItem = {
		id: 'precheck' | 'execute' | 'verify' | 'audit';
		label: string;
		state: WorkflowStage;
	};

	interface Props {
		flowSections: FlowSection[];
		glossary: GlossaryItem[];
		autonomyActions: AutonomyAction[];
		autonomyWorkflow: WorkflowItem[];
		autonomyBusyId: string;
		autonomyMessage: string;
		onSelectTab: (tab: string) => void;
		onRunAutonomy: (actionId: string) => void;
		runningLabel: string;
		workflowTitle: string;
	}

	let {
		flowSections,
		glossary,
		autonomyActions,
		autonomyWorkflow,
		autonomyBusyId,
		autonomyMessage,
		onSelectTab,
		onRunAutonomy,
		runningLabel,
		workflowTitle
	}: Props = $props();
</script>

<section class="card rounded-3xl border border-white/5 bg-base-200/70 p-6 sm:p-8 space-y-6">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<div class="flex items-center gap-2 text-sm font-black tracking-widest uppercase opacity-60">
				<Layers3 size={15} />
				Mission Architecture
			</div>
			<div class="mt-1 text-xs opacity-50">Operational lanes, autonomy actions, and team glossary.</div>
		</div>
		<div class="badge badge-outline">{flowSections.length} lanes</div>
	</div>

	<div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
		{#each flowSections as section (section.id)}
			<div class="rounded-2xl border border-white/10 bg-base-100/45 p-4 space-y-3">
				<button class="flex items-center gap-2 text-sm font-black uppercase tracking-wide hover:text-primary" onclick={() => onSelectTab(section.tab)}>
					<Layers3 size={14} class="text-primary" />
					{section.title}
				</button>
				<div class="text-xs opacity-65">{section.detail}</div>
				<div class="flex flex-wrap gap-2">
					{#each section.subsections as sub (sub)}
						<span class="badge badge-ghost border border-white/10 px-3 py-2 text-[11px]">{sub}</span>
					{/each}
				</div>
			</div>
		{/each}
	</div>

	<div class="grid grid-cols-1 gap-4 xl:grid-cols-[1.2fr_1fr]">
		<div class="rounded-2xl border border-white/10 bg-base-100/40 p-4 space-y-3">
			<div class="flex items-center gap-2 text-sm font-black uppercase tracking-wide">
				<Bot size={15} class="text-primary" />
				Autonomy Actions
			</div>
			<div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
				{#each autonomyActions as action (action.id)}
					<div class="rounded-xl border p-3 text-xs space-y-2 {action.tone === 'error' ? 'border-error/30 bg-error/10' : action.tone === 'warning' ? 'border-warning/30 bg-warning/10' : action.tone === 'success' ? 'border-success/30 bg-success/10' : 'border-info/30 bg-info/10'}">
						<div class="font-black uppercase tracking-wide">{action.title}</div>
						<div class="opacity-70">{action.detail}</div>
						<button class="btn btn-xs btn-outline" onclick={() => onRunAutonomy(action.id)} disabled={autonomyBusyId !== ''}>
							{#if autonomyBusyId === action.id}
								<RefreshCw size={12} class="animate-spin" />
								{runningLabel}
							{:else}
								<Rocket size={12} />
								{action.cta}
							{/if}
						</button>
					</div>
				{/each}
			</div>

			<div class="rounded-xl border border-white/10 bg-base-100/40 p-3 text-xs space-y-2">
				<div class="flex items-center gap-2 font-black uppercase tracking-wide opacity-70">
					<Wrench size={12} />
					{workflowTitle}
				</div>
				<div class="flex flex-wrap gap-2">
					{#each autonomyWorkflow as step (step.id)}
						<div class="badge px-3 py-2 {step.state === 'done' ? 'badge-success' : step.state === 'failed' ? 'badge-error' : step.state === 'running' ? 'badge-warning' : 'badge-ghost'}">
							{step.label}
						</div>
					{/each}
				</div>
				{#if autonomyMessage}
					<div class="opacity-70">{autonomyMessage}</div>
				{/if}
			</div>
		</div>

		<div class="rounded-2xl border border-white/10 bg-base-100/40 p-4 space-y-3">
			<div class="flex items-center gap-2 text-sm font-black uppercase tracking-wide">
				<BookOpen size={15} class="text-primary" />
				Definitions
			</div>
			<div class="space-y-2">
				{#each glossary as item (item.term)}
					<div class="rounded-xl border border-white/10 bg-base-100/40 p-3 text-xs">
						<div class="font-black uppercase tracking-wide">{item.term}</div>
						<div class="mt-1 opacity-70">{item.meaning}</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
</section>
