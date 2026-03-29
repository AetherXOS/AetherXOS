<script lang="ts">
	import { resolve } from '$app/paths';
	import { appState } from '$lib/state.svelte';
	import {
		AlertTriangle,
		ArrowRight,
		BookOpen,
		CheckCircle,
		Compass,
		Key,
		Layers3,
		Link2Off,
		RefreshCw,
		Rocket,
		Server,
		Settings
	} from 'lucide-svelte';
	import { orchestrator } from '$lib/services/orchestrator';

	let retrying = $state(false);

	type StepHref = '/settings';
	type Step = { icon: typeof Server; label: string; detail: string; done: boolean; href?: StepHref };

	const steps = $derived.by((): Step[] => [
		{
			icon: Server,
			label: 'Agent URL',
			detail: appState.agentUrl ? appState.agentUrl : 'Not set — open Settings to configure',
			done: Boolean(appState.agentUrl),
			href: '/settings'
		},
		{
			icon: Key,
			label: 'Auth token',
			detail: appState.agentToken ? '••••••••' : 'No token set — may be required by agent',
			done: Boolean(appState.agentToken),
			href: '/settings'
		},
		{
			icon: Link2Off,
			label: 'Agent reachable',
			detail: appState.isConnected
				? 'Connected successfully'
				: 'Agent not responding — check URL and network',
			done: appState.isConnected
		},
		{
			icon: RefreshCw,
			label: 'Resilience state',
			detail: appState.apiCircuitOpen
				? 'Circuit open — waiting cooldown before retrying requests'
				: appState.retryCount > 0
					? `Recovering with retry attempts (${appState.retryCount})`
					: `Stable link (${Math.round(appState.latencyMs)}ms)` ,
			done: appState.isConnected && !appState.apiCircuitOpen
		}
	]);

	const completed = $derived(steps.filter((x) => x.done).length);
	const progress = $derived(Math.round((completed / steps.length) * 100));
	const currentLayer = $derived.by(() => {
		if (!appState.agentUrl) return 'Layer 1 - Connection Setup';
		if (!appState.agentToken) return 'Layer 2 - Authentication Setup';
		if (!appState.isConnected) return 'Layer 3 - Reachability Check';
		return 'Layer 4 - Operational Flow';
	});

	async function retryNow() {
		retrying = true;
		try {
			await orchestrator.sync(true);
		} finally {
			retrying = false;
		}
	}
</script>

<section
	class="rounded-2xl border p-4 sm:p-5 {appState.isConnected
		? 'border-success/25 bg-success/8'
		: 'border-warning/25 bg-warning/8'}"
	aria-live="polite"
>
	<div class="space-y-4">
		<div class="flex flex-wrap items-center gap-3">
			<div class="flex items-center gap-2 rounded-xl bg-base-100/70 px-3 py-1.5 text-[11px] font-black tracking-wide uppercase">
				<Layers3 size={13} class="opacity-70" />
				Guidance Layer
			</div>
			<div class="text-xs font-bold opacity-70">{currentLayer}</div>
			<div class="ml-auto text-xs font-mono opacity-60">Progress {progress}%</div>
		</div>

		<div class="h-2 overflow-hidden rounded-full bg-base-100/70">
			<div class="h-full rounded-full transition-all duration-700 {appState.isConnected ? 'bg-success' : 'bg-warning'}" style={`width: ${progress}%`}></div>
		</div>

		<div class="grid grid-cols-1 gap-4 lg:grid-cols-[1.1fr_1fr]">
			<div class="space-y-3">
				<div class="flex items-start gap-3">
					<div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg {appState.isConnected ? 'bg-success/15' : 'bg-warning/15'}">
						{#if appState.isConnected}
							<Compass size={16} class="text-success" />
						{:else}
							<AlertTriangle size={16} class="text-warning" />
						{/if}
					</div>
					<div>
						<div class="text-sm font-black tracking-wide {appState.isConnected ? 'text-success' : 'text-warning'}">
							{appState.isConnected ? 'Connection established' : 'Agent not connected'}
						</div>
						<div class="mt-0.5 text-xs opacity-60">
							{appState.isConnected
								? 'You can proceed with monitoring, controls, and live incident response flows.'
								: 'Follow this checklist. Each completed step unlocks the next operational layer.'}
						</div>
					</div>
				</div>

				<ol class="space-y-2">
					{#each steps as step, i (i)}
						{@const Icon = step.icon}
						<li class="flex items-center gap-3 text-xs">
							<div class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full {step.done ? 'bg-success/20' : 'bg-base-300'}">
								{#if step.done}
									<CheckCircle size={12} class="text-success" />
								{:else}
									<Icon size={11} class="opacity-50" />
								{/if}
							</div>
							<span class="font-bold {step.done ? 'opacity-40 line-through' : ''}">{step.label}</span>
							<span class="opacity-50">-</span>
							<span class="opacity-60 truncate">{step.detail}</span>
							{#if !step.done && step.href}
								<a href={resolve(step.href)} class="ml-auto shrink-0 flex items-center gap-1 text-primary hover:underline">
									Fix <ArrowRight size={11} />
								</a>
							{/if}
						</li>
					{/each}
				</ol>

				<div class="rounded-xl border border-white/10 bg-base-100/45 px-3 py-2 text-[11px] opacity-70">
					<div>Quality: {appState.connectionQuality}</div>
					<div>Last sync: {appState.lastSyncAt ? new Date(appState.lastSyncAt).toLocaleTimeString() : 'never'}</div>
					{#if appState.lastSyncError}
						<div class="text-warning">Last error: {appState.lastSyncError}</div>
					{/if}
				</div>
			</div>

			<div class="rounded-xl border border-white/10 bg-base-100/45 p-3">
				<div class="mb-2 flex items-center gap-2 text-[11px] font-black tracking-wide uppercase opacity-60">
					<BookOpen size={13} />
					Next Actions
				</div>
				<div class="space-y-2 text-xs">
					{#if !appState.isConnected}
						<a href={resolve('/settings#connection')} class="btn btn-sm btn-warning w-full justify-start gap-2">
							<Settings size={13} />
							Configure connection
						</a>
						<button class="btn btn-sm btn-ghost w-full justify-start gap-2" onclick={retryNow} disabled={retrying}>
							<RefreshCw size={13} class={retrying ? 'animate-spin' : ''} />
							{retrying ? 'Retrying...' : 'Retry connection'}
						</button>
						<a href={resolve('/settings#authentication')} class="btn btn-sm btn-ghost w-full justify-start gap-2">
							<Key size={13} />
							Set authentication token
						</a>
					{:else}
						<a href={resolve('/executive')} class="btn btn-sm btn-success w-full justify-start gap-2">
							<Rocket size={13} />
							Go to Executive overview
						</a>
						<a href={resolve('/operations')} class="btn btn-sm btn-ghost w-full justify-start gap-2">
							<Compass size={13} />
							Open Operations runbook
						</a>
						<a href={resolve('/control-center')} class="btn btn-sm btn-ghost w-full justify-start gap-2">
							<Server size={13} />
							Manage jobs in Control Center
						</a>
					{/if}
				</div>
			</div>
		</div>
	</div>
</section>
