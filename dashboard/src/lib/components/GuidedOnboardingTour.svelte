<script lang="ts">
	import { browser } from '$app/environment';
	import { resolve } from '$app/paths';
	import {
		ArrowLeft,
		ArrowRight,
		CheckCircle,
		ChevronsRight,
		Compass,
		LayoutDashboard,
		Minimize2,
		Rocket,
		Settings,
		Terminal
	} from 'lucide-svelte';
	import { onMount } from 'svelte';

	type TourStep = {
		id: string;
		title: string;
		detail: string;
		href: '/settings#connection' | '/executive' | '/operations' | '/control-center';
		icon: typeof Rocket;
	};

	const TOUR_KEY = 'dashboard.guidedTour.completed.v1';

	const steps: TourStep[] = [
		{
			id: 'settings',
			title: 'Configure your system',
			detail: 'Start from Settings and complete connection plus authentication layers.',
			href: '/settings#connection',
			icon: Settings
		},
		{
			id: 'executive',
			title: 'Read executive overview',
			detail: 'Verify live telemetry and subsystem health cards from one place.',
			href: '/executive',
			icon: LayoutDashboard
		},
		{
			id: 'operations',
			title: 'Follow operations runbook',
			detail: 'Detect, triage, and resolve incidents with guided flow actions.',
			href: '/operations',
			icon: Compass
		},
		{
			id: 'control',
			title: 'Operate control center',
			detail: 'Run and monitor jobs after baseline connectivity is stable.',
			href: '/control-center',
			icon: Terminal
		}
	];

	let open = $state(false);
	let expanded = $state(true);
	let index = $state(0);

	const current = $derived(steps[index]);
	const progress = $derived(Math.round(((index + 1) / steps.length) * 100));

	onMount(() => {
		if (!browser) return;
		const done = window.localStorage.getItem(TOUR_KEY) === '1';
		open = !done;
	});

	function closeTour(markDone = true) {
		open = false;
		expanded = false;
		if (browser && markDone) {
			window.localStorage.setItem(TOUR_KEY, '1');
		}
	}

	function restartTour() {
		index = 0;
		open = true;
		expanded = true;
		if (browser) {
			window.localStorage.removeItem(TOUR_KEY);
		}
	}

	function next() {
		if (index >= steps.length - 1) {
			closeTour(true);
			return;
		}
		index += 1;
	}

	function prev() {
		if (index > 0) index -= 1;
	}
</script>

{#if open}
	<div class="fixed right-4 bottom-4 z-90 w-[min(30rem,calc(100vw-2rem))]">
		<div class="rounded-3xl border border-primary/25 bg-base-100/95 shadow-2xl backdrop-blur-xl">
			<div class="bg-primary/12 flex items-center justify-between rounded-t-3xl border-b border-primary/20 px-4 py-3">
				<div class="flex items-center gap-2">
					<div class="bg-primary text-primary-content flex h-8 w-8 items-center justify-center rounded-xl shadow-lg">
						<Rocket size={14} />
					</div>
					<div>
						<div class="text-[11px] font-black tracking-widest uppercase opacity-70">Guided Tour</div>
						<div class="text-xs opacity-65">{progress}% complete</div>
					</div>
				</div>
				<div class="flex items-center gap-2">
					<button class="btn btn-xs btn-ghost" onclick={() => (expanded = !expanded)} title={expanded ? 'Collapse' : 'Expand'}>
						{#if expanded}<Minimize2 size={13} />{:else}<ChevronsRight size={13} />{/if}
					</button>
					<button class="btn btn-xs btn-ghost" onclick={() => closeTour(true)} title="Dismiss guide">✕</button>
				</div>
			</div>

			{#if expanded}
				<div class="p-4 space-y-4">
					<div class="h-2 overflow-hidden rounded-full bg-base-200">
						<div class="bg-primary h-full rounded-full transition-all duration-500" style={`width: ${progress}%`}></div>
					</div>

					<div class="grid grid-cols-[9rem_1fr] gap-4">
						<div class="space-y-2 pr-2 border-r border-white/10">
							{#each steps as step, i (step.id)}
								<button
									class="w-full rounded-xl border px-2 py-2 text-left text-[11px] font-bold transition-all {i === index ? 'border-primary/40 bg-primary/10 text-primary' : i < index ? 'border-success/30 bg-success/10' : 'border-white/10 bg-base-200/40 hover:border-primary/20'}"
									onclick={() => (index = i)}
								>
									<div class="truncate">{i + 1}. {step.title}</div>
								</button>
							{/each}
						</div>

						<div class="rounded-2xl border border-white/10 bg-base-200/60 p-4">
							<div class="flex items-center gap-2 text-sm font-black uppercase tracking-wide">
								<current.icon size={16} class="text-primary" />
								{current.title}
							</div>
							<div class="mt-2 text-xs opacity-70 leading-relaxed">{current.detail}</div>
							<a href={resolve(current.href)} class="btn btn-sm btn-primary mt-4 gap-2 w-full sm:w-auto">
								Open this step
								<ArrowRight size={14} />
							</a>
						</div>
					</div>

					<div class="flex items-center justify-between gap-2">
						<div class="flex gap-2">
							<button class="btn btn-sm btn-ghost gap-2" onclick={prev} disabled={index === 0}>
								<ArrowLeft size={14} />
								Back
							</button>
							<button class="btn btn-sm btn-outline" onclick={() => closeTour(true)}>Hide tour</button>
						</div>
						<button class="btn btn-sm btn-primary gap-2" onclick={next}>
							{index === steps.length - 1 ? 'Finish' : 'Next'}
							{#if index === steps.length - 1}
								<CheckCircle size={14} />
							{:else}
								<ArrowRight size={14} />
							{/if}
						</button>
					</div>
				</div>
			{/if}
		</div>
	</div>
{:else}
	<button class="btn btn-sm btn-primary fixed right-4 bottom-4 z-40 gap-1 rounded-2xl shadow-lg" onclick={restartTour}>
		<Rocket size={12} />
		Start guided tour
	</button>
{/if}
